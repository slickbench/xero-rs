use core::fmt;
use std::borrow::Cow;
use std::str::FromStr;
use std::time::Duration;

use oauth2::{
    AccessToken, AuthorizationCode, CsrfToken, HttpClientError, RefreshToken, TokenResponse,
};
use reqwest::{header, IntoUrl, Method, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::time::sleep;
use url::Url;
use uuid::Uuid;

use crate::error::{self, Error, Result};
use crate::oauth::{KeyPair, OAuthClient};
use crate::scope::Scope;
use crate::endpoints::XeroEndpoint;
use crate::entities::{
    contact::{self, Contact},
    invoice::{self, Invoice},
    purchase_order::{self, PurchaseOrder},
    quote::{self, Quote},
    timesheet::{self, PostTimesheet, Timesheet},
    MutationResponse,
};
use crate::payroll::{
    employee::{self, Employee},
    settings::{
        earnings_rates::{self, EarningsRate},
        pay_calendar::{self, PayCalendar},
    },
};

const XERO_AUTH_URL: &str = "https://login.xero.com/identity/connect/authorize";
const XERO_TOKEN_URL: &str = "https://identity.xero.com/connect/token";
const MAX_RETRY_ATTEMPTS: usize = 3;

// Rate limiting headers used by the Xero API
/// Header containing number of remaining daily API calls
const HEADER_DAY_LIMIT_REMAINING: &str = "X-DayLimit-Remaining";
/// Header containing number of remaining per-minute API calls
const HEADER_MIN_LIMIT_REMAINING: &str = "X-MinLimit-Remaining";
/// Header containing number of remaining app-wide per-minute API calls
const HEADER_APP_MIN_LIMIT_REMAINING: &str = "X-AppMinLimit-Remaining";
/// Header identifying which rate limit was hit when a 429 is returned
const HEADER_RATE_LIMIT_PROBLEM: &str = "X-Rate-Limit-Problem";

#[derive(Debug, Clone)]
/// Information about the remaining API rate limits
///
/// Xero applies several rate limits to API usage:
/// - Daily limit: 5000 calls per day per tenant
/// - Minute limit: 60 calls per minute per tenant
/// - App minute limit: 10,000 calls per minute across all tenants
pub struct RateLimitInfo {
    /// Number of remaining API calls for the day (out of 5000)
    pub day_limit_remaining: Option<u32>,
    /// Number of remaining API calls for the minute (out of 60)
    pub minute_limit_remaining: Option<u32>,
    /// Number of remaining API calls for the app across all tenants (out of 10,000)
    pub app_minute_limit_remaining: Option<u32>,
}

impl Default for RateLimitInfo {
    fn default() -> Self {
        Self {
            day_limit_remaining: None,
            minute_limit_remaining: None,
            app_minute_limit_remaining: None,
        }
    }
}

impl RateLimitInfo {
    /// Extract rate limit information from response headers
    fn from_response_headers(headers: &header::HeaderMap) -> Self {
        Self {
            day_limit_remaining: headers
                .get(HEADER_DAY_LIMIT_REMAINING)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u32>().ok()),
            minute_limit_remaining: headers
                .get(HEADER_MIN_LIMIT_REMAINING)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u32>().ok()),
            app_minute_limit_remaining: headers
                .get(HEADER_APP_MIN_LIMIT_REMAINING)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u32>().ok()),
        }
    }
    
    /// Returns true if any of the limits are close to being exhausted
    pub fn is_near_limit(&self) -> bool {
        self.day_limit_remaining.map_or(false, |v| v < 100) ||
        self.minute_limit_remaining.map_or(false, |v| v < 10) ||
        self.app_minute_limit_remaining.map_or(false, |v| v < 100)
    }
    
    /// Log current rate limit status if getting close to limits
    pub fn log_if_near_limit(&self) {
        if self.is_near_limit() {
            tracing::warn!(
                "Approaching Xero API rate limits: day={:?}, minute={:?}, app_minute={:?}", 
                self.day_limit_remaining,
                self.minute_limit_remaining,
                self.app_minute_limit_remaining
            );
        }
    }
}

#[allow(unused)]
#[derive(Clone, Debug)]
/// This is the client that is used for interacting with the Xero API. It handles OAuth 2 authentication
/// and context (the current tenant).
pub struct Client {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
    tenant_id: Option<Uuid>,
    /// Information about API rate limits from the latest API response
    ///
    /// This field is updated with rate limit information from each successful API call.
    /// It can be used to monitor when you're approaching rate limits to implement preemptive
    /// throttling or backoff strategies.
    rate_limit_info: RateLimitInfo,
}

impl Client {
    #[instrument(skip(self))]
    fn build_http_client(&self) -> reqwest::Client {
        let mut headers = header::HeaderMap::new();
        headers.append(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {}", self.access_token.secret()))
                .unwrap(),
        );
        if let Some(tenant_id) = self.tenant_id {
            headers.append(
                "Xero-tenant-id",
                header::HeaderValue::from_str(&tenant_id.to_string()).unwrap(),
            );
        }
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap()
    }

    #[instrument]
    fn build_oauth_client(key_pair: KeyPair) -> OAuthClient {
        let client = oauth2::Client::new(key_pair.0);

        let client = client
            .set_auth_uri(oauth2::AuthUrl::new(XERO_AUTH_URL.to_string()).unwrap())
            .set_token_uri(oauth2::TokenUrl::new(XERO_TOKEN_URL.to_string()).unwrap());

        match key_pair.1 {
            Some(secret) => client.set_client_secret(secret),
            None => client,
        }
    }

    /// Generates an authorization URL to use for the code flow authorization method.
    #[instrument(skip(scopes))]
    pub fn authorize_url(
        key_pair: KeyPair,
        redirect_url: Url,
        scopes: impl Into<Scope>,
    ) -> (Url, CsrfToken) {
        let scope = scopes.into();
        Self::build_oauth_client(key_pair)
            .set_redirect_uri(oauth2::RedirectUrl::from_url(redirect_url))
            .authorize_url(CsrfToken::new_random)
            .add_scopes(vec![scope.into_oauth2()])
            .url()
    }

    /// # Errors
    /// Returns an error if the connection can't be made.
    #[instrument(skip(scopes))]
    pub async fn from_client_credentials(
        key_pair: KeyPair,
        scopes: impl Into<Option<Scope>>,
    ) -> std::result::Result<
        Self,
        oauth2::RequestTokenError<HttpClientError<reqwest::Error>, error::OAuth2ErrorResponse>,
    > {
        let scopes = scopes.into();
        let client = Self::build_oauth_client(key_pair);
        let http_client = reqwest::Client::new();

        let mut request = client.exchange_client_credentials();
        
        if let Some(scope) = scopes {
            request = request.add_scopes(vec![scope.into_oauth2()]);
        }

        let token = request.request_async(&http_client).await?;

        let access_token = token.access_token().clone();
        let refresh_token = token.refresh_token().cloned();

        Ok(Self {
            access_token,
            refresh_token,
            tenant_id: None,
            rate_limit_info: RateLimitInfo::default(),
        })
    }

    /// Creates an authorized client from a code generated in the code flow authorization method.
    ///
    /// # Errors
    /// Returns an error if the connection can't be made.
    #[instrument]
    pub async fn from_authorization_code(
        key_pair: KeyPair,
        redirect_url: Url,
        code: String,
    ) -> std::result::Result<
        Self,
        oauth2::RequestTokenError<HttpClientError<reqwest::Error>, error::OAuth2ErrorResponse>,
    > {
        let oauth_client = Self::build_oauth_client(key_pair.clone());
        let http_client = reqwest::Client::new();

        let token_result = oauth_client
            .exchange_code(AuthorizationCode::new(code))
            .set_redirect_uri(Cow::Owned(oauth2::RedirectUrl::from_url(redirect_url)))
            .request_async(&http_client)
            .await?;

        Ok(Self {
            access_token: token_result.access_token().clone(),
            refresh_token: token_result.refresh_token().cloned(),
            tenant_id: None,
            rate_limit_info: RateLimitInfo::default(),
        })
    }

    /// Refreshes the access token using the refresh token.
    pub async fn refresh_access_token(&mut self, key_pair: KeyPair) -> Result<()> {
        let oauth_client = Self::build_oauth_client(key_pair);
        let http_client = reqwest::Client::new();

        if let Some(refresh_token) = &self.refresh_token {
            let token_result = oauth_client
                .exchange_refresh_token(refresh_token)
                .request_async(&http_client)
                .await
                .map_err(Error::OAuth2)?;

            self.access_token = token_result.access_token().clone();
            if let Some(new_refresh_token) = token_result.refresh_token() {
                self.refresh_token = Some(new_refresh_token.clone());
            }
        }
        Ok(())
    }

    /// Sets the tenant ID for this client.
    pub fn set_tenant(&mut self, tenant_id: Option<Uuid>) {
        trace!(?tenant_id, "updating tenant id");
        self.tenant_id = tenant_id;
    }

    /// Build a request object with authentication headers.
    pub(crate) fn build_request<U: IntoUrl + fmt::Debug>(
        &self,
        method: Method,
        url: U,
    ) -> RequestBuilder {
        self.build_http_client()
            .request(method, url)
            .header(header::ACCEPT, "application/json")
    }

    /// Get the current rate limit information
    pub fn rate_limit_info(&self) -> &RateLimitInfo {
        &self.rate_limit_info
    }

    /// Execute a request with automatic retry for rate limit errors
    async fn execute_with_retry<T, F, Fut>(&self, request_fn: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        
        loop {
            attempts += 1;
            
            match request_fn().await {
                Ok(result) => return Ok(result),
                Err(Error::RateLimitExceeded { retry_after, .. }) if attempts <= MAX_RETRY_ATTEMPTS => {
                    let wait_time = retry_after.unwrap_or(Duration::from_secs(60));
                    
                    tracing::warn!(
                        "Rate limit exceeded (attempt {}/{}), waiting for {:?} before retrying",
                        attempts,
                        MAX_RETRY_ATTEMPTS,
                        wait_time
                    );
                    
                    // Wait for the specified time before retrying
                    sleep(wait_time).await;
                    continue;
                },
                Err(e) => return Err(e),
            }
        }
    }

    /// Perform an authenticated `GET` request against the API with automatic retry.
    #[instrument(skip(self, query))]
    pub async fn get<
        'a,
        R: DeserializeOwned,
        U: IntoUrl + fmt::Debug + Clone,
        T: Serialize + Sized + fmt::Debug,
    >(
        &self,
        url: U,
        query: &T,
    ) -> Result<R> {
        self.execute_with_retry(|| async {
            trace!(?query, ?url, "making GET request");
            let response = self.build_request(Method::GET, url.clone())
                .query(query)
                .send()
                .await?;
                
            Self::handle_response(response).await
        }).await
    }

    /// Perform a `GET` request against the API using a typed XeroEndpoint with automatic retry.
    #[instrument(skip(self, query))]
    pub async fn get_endpoint<
        'a,
        R: DeserializeOwned,
        T: Serialize + Sized + fmt::Debug,
    >(
        &self,
        endpoint: XeroEndpoint,
        query: &T,
    ) -> Result<R> {
        self.execute_with_retry(|| async {
            trace!(?query, endpoint = ?endpoint, "making GET request with endpoint");
            let url = endpoint.to_url()?;
            let response = self.build_request(Method::GET, url)
                .query(query)
                .send()
                .await?;
                
            Self::handle_response(response).await
        }).await
    }

    /// Perform an authenticated `PUT` request against the API with automatic retry.
    #[instrument(skip(self, data))]
    pub async fn put<
        'a, 
        R: DeserializeOwned, 
        U: IntoUrl + fmt::Debug + Clone, 
        T: Serialize + Sized
    >(
        &self,
        url: U,
        data: &T,
    ) -> Result<R> {
        self.execute_with_retry(|| async {
            trace!(json = ?serde_json::to_string(data).unwrap(), ?url, "making PUT request");
            let response = self.build_request(Method::PUT, url.clone())
                .json(data)
                .send()
                .await?;
                
            Self::handle_response(response).await
        }).await
    }

    /// Perform an authenticated `POST` request against the API with automatic retry.
    #[instrument(skip(self, data))]
    pub async fn post<
        'a,
        R: DeserializeOwned,
        U: IntoUrl + fmt::Debug + Clone,
        T: Serialize + Sized + fmt::Debug,
    >(
        &self,
        url: U,
        data: &T,
    ) -> Result<R> {
        self.execute_with_retry(|| async {
            trace!(json = ?serde_json::to_string(data).unwrap(), ?url, "making POST request");
            let response = self.build_request(Method::POST, url.clone())
                .json(data)
                .send()
                .await?;
                
            Self::handle_response(response).await
        }).await
    }

    /// Perform a `POST` request against the API using a typed XeroEndpoint with automatic retry.
    #[instrument(skip(self, data))]
    pub async fn post_endpoint<'a, R: DeserializeOwned, T: Serialize + Sized + fmt::Debug>(
        &self,
        endpoint: XeroEndpoint,
        data: &T,
    ) -> Result<R> {
        self.execute_with_retry(|| async {
            trace!(json = ?serde_json::to_string(data).unwrap(), endpoint = ?endpoint, "making POST request with endpoint");
            let url = endpoint.to_url()?;
            let response = self.build_request(Method::POST, url)
                .json(data)
                .send()
                .await?;
                
            Self::handle_response(response).await
        }).await
    }

    /// Perform a `PUT` request against the API using a typed XeroEndpoint with automatic retry.
    #[instrument(skip(self, data))]
    pub async fn put_endpoint<'a, R: DeserializeOwned, T: Serialize + Sized>(
        &self,
        endpoint: XeroEndpoint,
        data: &T,
    ) -> Result<R> {
        self.execute_with_retry(|| async {
            trace!(json = ?serde_json::to_string(data).unwrap(), endpoint = ?endpoint, "making PUT request with endpoint");
            let url = endpoint.to_url()?;
            let response = self.build_request(Method::PUT, url)
                .json(data)
                .send()
                .await?;
                
            Self::handle_response(response).await
        }).await
    }

    /// Perform an authenticated `DELETE` request against the API with automatic retry.
    #[instrument(skip(self))]
    pub async fn delete<U: IntoUrl + fmt::Debug + Clone>(&self, url: U) -> Result<()> {
        self.execute_with_retry(|| async {
            trace!(?url, "making DELETE request");
            let response = self.build_request(Method::DELETE, url.clone()).send().await?;
            
            if response.status() == StatusCode::NO_CONTENT || response.status() == StatusCode::OK {
                Ok(())
            } else {
                let content_length = response.content_length().unwrap_or(0);
                if content_length == 0 {
                    Err(Error::Request(response.error_for_status().unwrap_err()))
                } else {
                    Err(Error::API(response.json().await?))
                }
            }
        }).await
    }

    /// Perform a `DELETE` request against the API using a typed XeroEndpoint with automatic retry.
    #[instrument(skip(self))]
    pub async fn delete_endpoint(&self, endpoint: XeroEndpoint) -> Result<()> {
        self.execute_with_retry(|| async {
            trace!(endpoint = ?endpoint, "making DELETE request with endpoint");
            let url = endpoint.to_url()?;
            let response = self.build_request(Method::DELETE, url).send().await?;
            
            if response.status() == StatusCode::NO_CONTENT {
                Ok(())
            } else {
                Self::handle_response::<()>(response).await
            }
        }).await
    }

    #[instrument(skip(response))]
    async fn handle_response<T: DeserializeOwned + Sized>(
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();
        let url = response.url().to_string();
        let entity_type = std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap_or("Unknown")
            .to_string();

        tracing::debug!(
            "Response from {}: status={}, entity_type={}",
            url,
            status,
            entity_type
        );
        
        // Extract rate limit information for logging
        let rate_limit_info = RateLimitInfo::from_response_headers(response.headers());
        
        // Log rate limit information if we're getting close to limits
        if rate_limit_info.is_near_limit() {
            tracing::warn!(
                "Approaching Xero API rate limits: day_remaining={:?}, minute_remaining={:?}, app_minute_remaining={:?}",
                rate_limit_info.day_limit_remaining,
                rate_limit_info.minute_limit_remaining,
                rate_limit_info.app_minute_limit_remaining
            );
        }
        
        // Handle rate limiting (429 Too Many Requests)
        if status == StatusCode::TOO_MANY_REQUESTS {
            // Extract rate limit headers
            let rate_limit_problem = response.headers()
                .get(HEADER_RATE_LIMIT_PROBLEM)
                .and_then(|v| v.to_str().ok())
                .map(String::from);
                
            let retry_after = response.headers()
                .get(header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(std::time::Duration::from_secs);
                
            // Log rate limit hit with detailed information
            tracing::warn!(
                "Rate limit exceeded for {}: problem={:?}, retry_after={:?}, day_remaining={:?}, minute_remaining={:?}, app_minute_remaining={:?}",
                url,
                rate_limit_problem,
                retry_after,
                rate_limit_info.day_limit_remaining,
                rate_limit_info.minute_limit_remaining,
                rate_limit_info.app_minute_limit_remaining
            );
            
            // Get response text for error context
            let text = response.text().await.unwrap_or_default();
            
            return Err(Error::RateLimitExceeded {
                retry_after,
                status_code: status,
                url,
                response_body: Some(text),
            });
        }

        let text = response.text().await?;
        
        // Only log brief info about response size at debug level
        tracing::debug!("Response body size: {} bytes", text.len());

        let handle_deserialize_error = {
            let text = text.clone();
            |e: serde_json::Error| {
                tracing::error!("Deserialization error: {}, near position: {} - response text around that position: {}", 
                    e, 
                    e.column(),
                    &text.chars().skip(e.column().saturating_sub(30)).take(100).collect::<String>()
                );
                Error::DeserializationError(e, Some(text))
            }
        };

        tracing::trace!("Response text:\n{}", text);
        match status {
            StatusCode::NOT_FOUND => Err(Error::NotFound {
                entity: entity_type,
                url,
                status_code: status,
                response_body: Some(text),
            }),
            status => match status {
                StatusCode::OK => match serde_json::from_str(&text) {
                    Ok(result) => Ok(result),
                    Err(e) => {
                        tracing::error!("Failed to deserialize response: {}", e);
                        Err(handle_deserialize_error(e))
                    }
                },
                StatusCode::FORBIDDEN => Err(Error::Forbidden(
                    serde_json::from_str(&text).map_err(handle_deserialize_error)?,
                )),
                _ => {
                    tracing::error!("Unexpected status code: {}", status);
                    Err(Error::API(
                        serde_json::from_str(&text).map_err(handle_deserialize_error)?,
                    ))
                }
            },
        }
    }

    /// Access the contacts API
    #[must_use]
    pub fn contacts(&self) -> ContactsApi {
        ContactsApi { client: self }
    }

    /// Access the invoices API
    #[must_use]
    pub fn invoices(&self) -> InvoicesApi {
        InvoicesApi { client: self }
    }

    /// Access the purchase orders API
    #[must_use]
    pub fn purchase_orders(&self) -> PurchaseOrdersApi {
        PurchaseOrdersApi { client: self }
    }

    /// Access the quotes API
    #[must_use]
    pub fn quotes(&self) -> QuotesApi {
        QuotesApi { client: self }
    }

    /// Access the timesheets API
    #[must_use]
    pub fn timesheets(&self) -> TimesheetsApi {
        TimesheetsApi { client: self }
    }
    
    /// Access the employees API
    #[must_use]
    pub fn employees(&self) -> EmployeesApi {
        EmployeesApi { client: self }
    }
    
    /// Access the earnings rates API
    #[must_use]
    pub fn earnings_rates(&self) -> EarningsRatesApi {
        EarningsRatesApi { client: self }
    }
    
    /// Access the pay calendars API
    #[must_use]
    pub fn pay_calendars(&self) -> PayCalendarsApi {
        PayCalendarsApi { client: self }
    }
}

/// API handler for Contacts endpoints
#[derive(Debug)]
pub struct ContactsApi<'a> {
    client: &'a Client,
}

impl ContactsApi<'_> {
    /// Retrieve a list of contacts
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<Contact>> {
        let empty_vec: Vec<String> = Vec::new();
        let response: contact::ListResponse = self.client.get_endpoint(XeroEndpoint::Contacts, &empty_vec).await?;
        Ok(response.contacts)
    }

    /// Retrieve a single contact by ID
    #[instrument(skip(self))]
    pub async fn get(&self, contact_id: Uuid) -> Result<Contact> {
        let endpoint = XeroEndpoint::Contact(contact_id);
        let empty_vec: Vec<String> = Vec::new();
        let response: contact::ListResponse = self.client.get_endpoint(endpoint.clone(), &empty_vec).await?;
        response.contacts.into_iter().next().ok_or(Error::NotFound {
            entity: "Contact".to_string(),
            url: endpoint.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some(format!("Contact with ID {contact_id} not found")),
        })
    }
}

/// API handler for Invoices endpoints
#[derive(Debug)]
pub struct InvoicesApi<'a> {
    client: &'a Client,
}

impl InvoicesApi<'_> {
    /// List invoices with optional parameters
    #[instrument(skip(self))]
    pub async fn list(&self, parameters: invoice::ListParameters) -> Result<Vec<Invoice>> {
        let response: invoice::ListResponse = self.client.get_endpoint(XeroEndpoint::Invoices, &parameters).await?;
        Ok(response.invoices)
    }

    /// List all invoices without any filtering
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> Result<Vec<Invoice>> {
        self.list(invoice::ListParameters::default()).await
    }

    /// Get a single invoice by ID
    #[instrument(skip(self))]
    pub async fn get(&self, invoice_id: Uuid) -> Result<Invoice> {
        let endpoint = XeroEndpoint::Invoice(invoice_id);
        let empty_tuple = ();
        let response: invoice::ListResponse = self.client.get_endpoint(endpoint.clone(), &empty_tuple).await?;
        response.invoices.into_iter().next().ok_or(Error::NotFound {
            entity: "Invoice".to_string(),
            url: endpoint.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some(format!("Invoice with ID {invoice_id} not found")),
        })
    }

    /// Create a new invoice
    #[instrument(skip(self, invoice))]
    pub async fn create(&self, invoice: &invoice::Builder) -> Result<Invoice> {
        // Create a request wrapper
        #[derive(Serialize, Debug)]
        #[serde(rename_all = "PascalCase")]
        struct InvoiceWrapper<'a> {
            invoices: Vec<&'a invoice::Builder>,
        }
        
        let request = InvoiceWrapper {
            invoices: vec![invoice],
        };

        let response: MutationResponse = self.client
            .put_endpoint(XeroEndpoint::Invoices, &request)
            .await?;

        // Extract invoice from response
        response
            .data
            .get_invoices()
            .and_then(|invoices| invoices.into_iter().next())
            .ok_or(Error::NotFound {
                entity: "Invoice".to_string(),
                url: XeroEndpoint::Invoices.to_string(),
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some("No invoice returned in response".to_string()),
            })
    }
    
    /// Update an existing invoice
    #[instrument(skip(self, invoice))]
    pub async fn update(&self, invoice_id: Uuid, invoice: &invoice::Builder) -> Result<Invoice> {
        invoice::update(self.client, invoice_id, invoice).await
    }
    
    /// Update or create an invoice
    #[instrument(skip(self, invoice))]
    pub async fn update_or_create(&self, invoice: &invoice::Builder) -> Result<Invoice> {
        invoice::update_or_create(self.client, invoice).await
    }
    
    /// Get the invoice as a PDF
    #[instrument(skip(self))]
    pub async fn get_pdf(&self, invoice_id: Uuid) -> Result<Vec<u8>> {
        invoice::get_pdf(self.client, invoice_id).await
    }
    
    /// Get an online invoice URL
    #[instrument(skip(self))]
    pub async fn get_online_invoice(&self, invoice_id: Uuid) -> Result<String> {
        invoice::get_online_invoice(self.client, invoice_id).await
    }
    
    /// Email the invoice to the contact
    #[instrument(skip(self))]
    pub async fn email(&self, invoice_id: Uuid) -> Result<()> {
        invoice::email(self.client, invoice_id).await
    }
    
    /// Get the history for an invoice
    #[instrument(skip(self))]
    pub async fn get_history(&self, invoice_id: Uuid) -> Result<Vec<invoice::HistoryRecord>> {
        invoice::get_history(self.client, invoice_id).await
    }
    
    /// Create a history record for an invoice
    #[instrument(skip(self))]
    pub async fn create_history(&self, invoice_id: Uuid, details: &str) -> Result<Vec<invoice::HistoryRecord>> {
        invoice::create_history(self.client, invoice_id, details).await
    }
    
    /// List attachments for an invoice
    #[instrument(skip(self))]
    pub async fn list_attachments(&self, invoice_id: Uuid) -> Result<Vec<invoice::Attachment>> {
        invoice::list_attachments(self.client, invoice_id).await
    }
    
    /// Get a specific attachment by ID
    #[instrument(skip(self))]
    pub async fn get_attachment(&self, invoice_id: Uuid, attachment_id: Uuid) -> Result<Vec<u8>> {
        invoice::get_attachment(self.client, invoice_id, attachment_id).await
    }
    
    /// Get an attachment by filename
    #[instrument(skip(self))]
    pub async fn get_attachment_by_filename(&self, invoice_id: Uuid, filename: &str) -> Result<Vec<u8>> {
        invoice::get_attachment_by_filename(self.client, invoice_id, filename).await
    }
    
    /// Upload an attachment to an invoice
    #[instrument(skip(self, attachment_content))]
    pub async fn upload_attachment(&self, invoice_id: Uuid, filename: &str, attachment_content: &[u8]) -> Result<invoice::Attachment> {
        invoice::upload_attachment(self.client, invoice_id, filename, attachment_content).await
    }
    
    /// Update an existing attachment
    #[instrument(skip(self, attachment_content))]
    pub async fn update_attachment(&self, invoice_id: Uuid, filename: &str, attachment_content: &[u8]) -> Result<invoice::Attachment> {
        invoice::update_attachment(self.client, invoice_id, filename, attachment_content).await
    }
}

/// API handler for Purchase Orders endpoints
#[derive(Debug)]
pub struct PurchaseOrdersApi<'a> {
    client: &'a Client,
}

impl PurchaseOrdersApi<'_> {
    /// Retrieve a list of purchase orders
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<PurchaseOrder>> {
        let empty_vec: Vec<String> = Vec::new();
        let response: purchase_order::ListResponse = self.client.get(purchase_order::ENDPOINT, &empty_vec).await?;
        Ok(response.purchase_orders)
    }

    /// Retrieve a single purchase order by ID
    #[instrument(skip(self))]
    pub async fn get(&self, purchase_order_id: Uuid) -> Result<PurchaseOrder> {
        let endpoint = Url::from_str(purchase_order::ENDPOINT)
            .and_then(|endpoint| endpoint.join(&purchase_order_id.to_string()))
            .map_err(|_| Error::InvalidEndpoint)?;
        let endpoint_str = endpoint.to_string();
        let empty_vec: Vec<String> = Vec::new();
        let response: purchase_order::ListResponse = self.client.get(endpoint, &empty_vec).await?;
        response
            .purchase_orders
            .into_iter()
            .next()
            .ok_or(Error::NotFound {
                entity: "PurchaseOrder".to_string(),
                url: endpoint_str,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!(
                    "Purchase Order with ID {purchase_order_id} not found"
                )),
            })
    }

    /// Create a new purchase order
    #[instrument(skip(self, builder))]
    pub async fn create(&self, builder: &purchase_order::Builder) -> Result<PurchaseOrder> {
        let result: MutationResponse = self.client.put(purchase_order::ENDPOINT, builder).await?;
        result
            .data
            .get_purchase_orders()
            .and_then(|po| po.into_iter().next())
            .ok_or(Error::NotFound {
                entity: "PurchaseOrder".to_string(),
                url: purchase_order::ENDPOINT.to_string(),
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(
                    "Failed to create purchase order - no purchase order in response".to_string(),
                ),
            })
    }
}

/// API handler for Quotes endpoints
#[derive(Debug)]
pub struct QuotesApi<'a> {
    client: &'a Client,
}

impl QuotesApi<'_> {
    /// Retrieve a list of quotes with filters
    #[instrument(skip(self, parameters))]
    pub async fn list(&self, parameters: quote::ListParameters) -> Result<Vec<Quote>> {
        quote::list(self.client, parameters).await
    }

    /// Retrieve a list of all quotes without filtering
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> Result<Vec<Quote>> {
        quote::list_all(self.client).await
    }

    /// Retrieve a single quote by ID
    #[instrument(skip(self))]
    pub async fn get(&self, quote_id: Uuid) -> Result<Quote> {
        quote::get(self.client, quote_id).await
    }
    
    /// Create a new quote
    #[instrument(skip(self, quote))]
    pub async fn create(&self, quote: &quote::QuoteBuilder) -> Result<Quote> {
        quote::create(self.client, quote).await
    }
    
    /// Update or create a quote
    #[instrument(skip(self, quote))]
    pub async fn update_or_create(&self, quote: &quote::QuoteBuilder) -> Result<Quote> {
        quote::update_or_create(self.client, quote).await
    }
    
    /// Update a specific quote
    #[instrument(skip(self, quote))]
    pub async fn update(&self, quote_id: Uuid, quote: &quote::QuoteBuilder) -> Result<Quote> {
        quote::update(self.client, quote_id, quote).await
    }
    
    /// Get the history records for a quote
    #[instrument(skip(self))]
    pub async fn get_history(&self, quote_id: Uuid) -> Result<Vec<quote::HistoryRecord>> {
        quote::get_history(self.client, quote_id).await
    }
    
    /// Create a history record for a quote
    #[instrument(skip(self))]
    pub async fn create_history(&self, quote_id: Uuid, details: &str) -> Result<Vec<quote::HistoryRecord>> {
        quote::create_history(self.client, quote_id, details).await
    }
    
    /// Get a quote as PDF
    #[instrument(skip(self))]
    pub async fn get_pdf(&self, quote_id: Uuid) -> Result<Vec<u8>> {
        quote::get_pdf(self.client, quote_id).await
    }
    
    /// List all attachments for a quote
    #[instrument(skip(self))]
    pub async fn list_attachments(&self, quote_id: Uuid) -> Result<Vec<quote::Attachment>> {
        quote::list_attachments(self.client, quote_id).await
    }
    
    /// Get a specific attachment by ID
    #[instrument(skip(self))]
    pub async fn get_attachment(&self, quote_id: Uuid, attachment_id: Uuid) -> Result<Vec<u8>> {
        quote::get_attachment(self.client, quote_id, attachment_id).await
    }
    
    /// Get an attachment by filename
    #[instrument(skip(self))]
    pub async fn get_attachment_by_filename(&self, quote_id: Uuid, filename: &str) -> Result<Vec<u8>> {
        quote::get_attachment_by_filename(self.client, quote_id, filename).await
    }
    
    /// Upload an attachment to a quote
    #[instrument(skip(self, attachment_content))]
    pub async fn upload_attachment(&self, quote_id: Uuid, filename: &str, attachment_content: &[u8]) -> Result<quote::Attachment> {
        quote::upload_attachment(self.client, quote_id, filename, attachment_content).await
    }
    
    /// Update an existing attachment
    #[instrument(skip(self, attachment_content))]
    pub async fn update_attachment(&self, quote_id: Uuid, filename: &str, attachment_content: &[u8]) -> Result<quote::Attachment> {
        quote::update_attachment(self.client, quote_id, filename, attachment_content).await
    }
}

/// API handler for Timesheets endpoints
#[derive(Debug)]
pub struct TimesheetsApi<'a> {
    client: &'a Client,
}

impl TimesheetsApi<'_> {
    /// Retrieve a list of timesheets with optional filtering
    ///
    /// # Parameters
    /// 
    /// * `parameters` - Optional filter parameters
    /// * `modified_after` - Optional ISO8601 timestamp (format: yyyy-mm-ddThh:mm:ss) to filter by modification date
    #[instrument(skip(self, parameters, modified_after))]
    pub async fn list(
        &self, 
        parameters: Option<timesheet::ListParameters>, 
        modified_after: Option<String>
    ) -> Result<Vec<Timesheet>> {
        Timesheet::list(self.client, parameters.as_ref(), modified_after).await
    }
    
    /// List all timesheets without any filtering
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> Result<Vec<Timesheet>> {
        self.list(None::<timesheet::ListParameters>, None).await
    }

    /// Retrieve a single timesheet by ID
    #[instrument(skip(self))]
    pub async fn get(&self, timesheet_id: Uuid) -> Result<Timesheet> {
        Timesheet::get(self.client, timesheet_id).await
    }

    /// Create a new timesheet
    #[instrument(skip(self, timesheet))]
    pub async fn create(&self, timesheet: &PostTimesheet) -> Result<Timesheet> {
        Timesheet::post(self.client, timesheet).await
    }
    
    /// Update a timesheet
    #[instrument(skip(self, timesheet))]
    pub async fn update(&self, timesheet: &Timesheet) -> Result<Timesheet> {
        Timesheet::update(self.client, timesheet).await
    }
}

/// API handler for Employees endpoints
#[derive(Debug)]
pub struct EmployeesApi<'a> {
    client: &'a Client,
}

impl EmployeesApi<'_> {
    /// Retrieve a list of employees
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<Employee>> {
        let empty_vec: Vec<String> = Vec::new();
        let response: employee::ListResponse = self.client.get(employee::ENDPOINT, &empty_vec).await?;
        Ok(response.employees)
    }
}

/// API handler for Earnings Rates endpoints
#[derive(Debug)]
pub struct EarningsRatesApi<'a> {
    client: &'a Client,
}

impl EarningsRatesApi<'_> {
    /// Retrieve a list of earnings rates
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<EarningsRate>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct PayItems {
            earnings_rates: Vec<EarningsRate>,
        }
        
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct ListResponse {
            pay_items: PayItems,
        }
        
        let empty_vec: Vec<String> = Vec::new();
        let response: ListResponse = self.client.get(earnings_rates::ENDPOINT, &empty_vec).await?;
        Ok(response.pay_items.earnings_rates)
    }
}

/// API client for interacting with Xero Payroll Calendars
///
/// This API provides methods for listing, retrieving, and creating pay calendars.
pub struct PayCalendarsApi<'a> {
    client: &'a Client,
}

impl PayCalendarsApi<'_> {
    /// Retrieve a list of pay calendars
    ///
    /// Returns all pay calendars defined in the Xero organization.
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails.
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<PayCalendar>> {
        let url = "https://api.xero.com/payroll.xro/1.0/PayrollCalendars";
        let response: pay_calendar::PayCalendarResponse = self.client.get(url, &()).await?;
        Ok(response.payroll_calendars)
    }
    
    /// Get a pay calendar by ID
    ///
    /// Retrieves a specific pay calendar using its unique identifier.
    ///
    /// # Arguments
    ///
    /// * `pay_calendar_id` - The UUID of the pay calendar to retrieve
    ///
    /// # Errors
    ///
    /// Returns an error if the pay calendar is not found or if the API request fails.
    #[instrument(skip(self))]
    pub async fn get(&self, pay_calendar_id: Uuid) -> Result<PayCalendar> {
        let url = format!("https://api.xero.com/payroll.xro/1.0/PayrollCalendars/{pay_calendar_id}");
        let response: pay_calendar::PayCalendarResponse = self.client.get(&url, &()).await?;
        
        if response.payroll_calendars.is_empty() {
            return Err(Error::NotFound {
                entity: "PayCalendar".to_string(),
                url,
                status_code: StatusCode::NOT_FOUND,
                response_body: Some(format!("Pay Calendar with ID {pay_calendar_id} not found")),
            });
        }
        
        Ok(response.payroll_calendars.into_iter().next().unwrap())
    }
    
    /// Create a new pay calendar
    ///
    /// Creates a new pay calendar with the specified details.
    ///
    /// # Arguments
    ///
    /// * `pay_calendar` - The pay calendar details to create
    ///
    /// # Errors
    ///
    /// Returns an error if the API request fails or if the response doesn't contain the created pay calendar.
    #[instrument(skip(self, pay_calendar))]
    pub async fn create(&self, pay_calendar: &pay_calendar::CreatePayCalendar) -> Result<PayCalendar> {
        let url = "https://api.xero.com/payroll.xro/1.0/PayrollCalendars";
        
        // Create a vector with a single pay calendar
        let request = vec![pay_calendar.clone()];
        
        let response: pay_calendar::PayCalendarResponse = self.client.post(url, &request).await?;
        
        if response.payroll_calendars.is_empty() {
            return Err(Error::NotFound {
                entity: "PayCalendar".to_string(),
                url: url.to_string(),
                status_code: StatusCode::OK,
                response_body: Some("No pay calendar was returned after creation".to_string()),
            });
        }
        
        Ok(response.payroll_calendars.into_iter().next().unwrap())
    }
}
