use core::fmt;
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use oauth2::{
    AccessToken, AuthorizationCode, CsrfToken, HttpClientError, RefreshToken, TokenResponse,
};
use reqwest::{IntoUrl, Method, RequestBuilder, StatusCode, header};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::RwLock;
use tokio::time::sleep;
use url::Url;
use uuid::Uuid;

use crate::endpoints::{BASE_URL, XeroEndpoint};
use crate::entities::{
    MutationResponse,
    account::{self, Account},
    contact::{self, Contact},
    invoice::{self, Invoice},
    item::{self, Item},
    purchase_order::{self, PurchaseOrder},
    quote::{self, Quote},
    timesheet::{self, PostTimesheet, Timesheet},
};
use crate::error::{self, Error, Result};
use crate::oauth::{KeyPair, OAuthClient};
use crate::payroll::{
    employee::{self, Employee},
    leave_application::{self, LeaveApplication, PostLeaveApplication},
    settings::{
        earnings_rates::{self, EarningsRate},
        leave_types::LeaveType,
        pay_calendar::{self, PayCalendar},
    },
};
use crate::scope::Scope;

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
#[derive(Default)]
pub struct RateLimitInfo {
    /// Number of remaining API calls for the day (out of 5000)
    pub day_limit_remaining: Option<u32>,
    /// Number of remaining API calls for the minute (out of 60)
    pub minute_limit_remaining: Option<u32>,
    /// Number of remaining API calls for the app across all tenants (out of 10,000)
    pub app_minute_limit_remaining: Option<u32>,
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
    #[must_use]
    pub fn is_near_limit(&self) -> bool {
        self.day_limit_remaining.is_some_and(|v| v < 100)
            || self.minute_limit_remaining.is_some_and(|v| v < 10)
            || self.app_minute_limit_remaining.is_some_and(|v| v < 100)
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

/// Encapsulates the mutable state that needs interior mutability
#[derive(Debug)]
struct TokenState {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
    /// When the access token expires (if known)
    expires_at: Option<std::time::Instant>,
    rate_limit_info: RateLimitInfo,
}

impl TokenState {
    /// Calculate expiry time from token response's expires_in duration
    fn calculate_expiry(expires_in: Option<std::time::Duration>) -> Option<std::time::Instant> {
        expires_in.map(|duration| std::time::Instant::now() + duration)
    }

    /// Check if the token is expired or will expire within the given margin
    fn is_expired_or_expiring(&self, margin: std::time::Duration) -> bool {
        self.expires_at
            .map(|expires_at| std::time::Instant::now() + margin >= expires_at)
            .unwrap_or(false)
    }
}

/// This is the client that is used for interacting with the Xero API. It handles OAuth 2 authentication
/// and context (the current tenant).
#[derive(Debug, Clone)]
pub struct Client {
    /// Mutable token state wrapped in Arc<RwLock> for interior mutability
    token_state: Arc<RwLock<TokenState>>,
    tenant_id: Arc<RwLock<Option<Uuid>>>,
    /// Optional credentials for automatic token refresh on 401 responses
    ///
    /// When set via `with_auto_refresh()`, the client will automatically attempt to
    /// refresh the access token if a request fails with an unauthorized error.
    refresh_credentials: Option<KeyPair>,
    /// Optional semaphore for limiting concurrent requests.
    ///
    /// Xero enforces a limit of 5 concurrent requests per organization.
    /// When set via `with_concurrency_limit()`, the client will ensure
    /// that no more than the specified number of requests are in flight.
    concurrency_limiter: Option<Arc<tokio::sync::Semaphore>>,
}

impl Client {
    #[instrument(skip(self))]
    async fn build_http_client(&self) -> reqwest::Client {
        let mut headers = header::HeaderMap::new();
        let token_state = self.token_state.read().await;
        headers.append(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {}", token_state.access_token.secret()))
                .unwrap(),
        );
        let tenant_id = self.tenant_id.read().await;
        if let Some(tenant_id) = *tenant_id {
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

    /// Extract and persist rate limit information from response headers
    async fn update_rate_limit_info(&self, headers: &reqwest::header::HeaderMap) {
        let info = RateLimitInfo::from_response_headers(headers);
        info.log_if_near_limit();

        let mut state = self.token_state.write().await;
        state.rate_limit_info = info;
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
        let expires_at = TokenState::calculate_expiry(token.expires_in());

        Ok(Self {
            token_state: Arc::new(RwLock::new(TokenState {
                access_token,
                refresh_token,
                expires_at,
                rate_limit_info: RateLimitInfo::default(),
            })),
            tenant_id: Arc::new(RwLock::new(None)),
            refresh_credentials: None,
            concurrency_limiter: None,
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
            token_state: Arc::new(RwLock::new(TokenState {
                access_token: token_result.access_token().clone(),
                refresh_token: token_result.refresh_token().cloned(),
                expires_at: TokenState::calculate_expiry(token_result.expires_in()),
                rate_limit_info: RateLimitInfo::default(),
            })),
            tenant_id: Arc::new(RwLock::new(None)),
            refresh_credentials: None,
            concurrency_limiter: None,
        })
    }

    /// Refreshes the access token using the refresh token.
    pub async fn refresh_access_token(&self, key_pair: KeyPair) -> Result<()> {
        let oauth_client = Self::build_oauth_client(key_pair);
        let http_client = reqwest::Client::new();

        let mut token_state = self.token_state.write().await;

        if let Some(refresh_token) = &token_state.refresh_token {
            let token_result = oauth_client
                .exchange_refresh_token(refresh_token)
                .request_async(&http_client)
                .await
                .map_err(Error::OAuth2)?;
            info!("Successfully refreshed access token");

            token_state.access_token = token_result.access_token().clone();
            token_state.expires_at = TokenState::calculate_expiry(token_result.expires_in());
            if let Some(new_refresh_token) = token_result.refresh_token() {
                token_state.refresh_token = Some(new_refresh_token.clone());
                info!("Successfully refreshed refresh token");
            }
        } else if let Some(_refresh_credentials) = &self.refresh_credentials {
            let token_result = oauth_client
                .exchange_client_credentials()
                .request_async(&http_client)
                .await
                .map_err(Error::OAuth2)?;
            info!("Successfully refreshed access token");
            token_state.access_token = token_result.access_token().clone();
            token_state.expires_at = TokenState::calculate_expiry(token_result.expires_in());
        } else {
            error!("No refresh token or credentials available");
        }
        Ok(())
    }

    /// Sets the tenant ID for this client.
    pub async fn set_tenant(&self, tenant_id: Option<Uuid>) {
        trace!(?tenant_id, "updating tenant id");
        let mut current_tenant = self.tenant_id.write().await;
        *current_tenant = tenant_id;
    }

    /// Proactively ensure the access token is valid before making requests.
    ///
    /// This method checks if the token is expired or will expire within 60 seconds,
    /// and automatically refreshes it if credentials are available.
    ///
    /// Unlike the automatic 401 retry mechanism, this prevents the initial failed
    /// request and provides more predictable behavior.
    ///
    /// # Errors
    ///
    /// Returns an error if token refresh is needed but fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xero_rs::{Client, KeyPair};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let key_pair = KeyPair::from_env();
    /// let client = Client::from_client_credentials(key_pair.clone(), None)
    ///     .await?
    ///     .with_auto_refresh(key_pair);
    ///
    /// // Proactively refresh before a batch of requests
    /// client.ensure_valid_token().await?;
    ///
    /// // Now make requests knowing the token is fresh
    /// let contacts = client.contacts().list_all().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn ensure_valid_token(&self) -> Result<()> {
        const REFRESH_MARGIN: std::time::Duration = std::time::Duration::from_secs(60);

        let needs_refresh = {
            let token_state = self.token_state.read().await;
            token_state.is_expired_or_expiring(REFRESH_MARGIN)
        };

        if needs_refresh {
            if let Some(key_pair) = &self.refresh_credentials {
                tracing::debug!("Token expiring soon, proactively refreshing");
                self.refresh_access_token(key_pair.clone()).await?;
                tracing::info!("Proactively refreshed access token");
            } else {
                tracing::warn!(
                    "Token is expiring but no refresh credentials available. \
                    Call with_auto_refresh() to enable automatic refresh."
                );
            }
        }

        Ok(())
    }

    /// Check if the current token is expired or will expire soon.
    ///
    /// Returns `true` if the token is expired or will expire within 60 seconds.
    /// Returns `false` if the token is valid or expiry is unknown.
    #[must_use]
    pub async fn is_token_expiring(&self) -> bool {
        const REFRESH_MARGIN: std::time::Duration = std::time::Duration::from_secs(60);
        let token_state = self.token_state.read().await;
        token_state.is_expired_or_expiring(REFRESH_MARGIN)
    }

    /// Enable automatic token refresh on 401 responses.
    ///
    /// When enabled, the client will automatically attempt to refresh the access token
    /// if a request fails with an unauthorized error, provided a refresh token is available.
    ///
    /// # Arguments
    ///
    /// * `key_pair` - The `OAuth2` credentials to use for refreshing the token
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xero_rs::{Client, KeyPair};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let key_pair = KeyPair::from_env();
    /// let client = Client::from_client_credentials(key_pair.clone(), None)
    ///     .await?
    ///     .with_auto_refresh(key_pair);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn with_auto_refresh(mut self, key_pair: KeyPair) -> Self {
        self.refresh_credentials = Some(key_pair);
        self
    }

    /// Disable automatic token refresh.
    ///
    /// This explicitly removes any stored credentials for automatic refresh.
    #[must_use]
    pub fn without_auto_refresh(mut self) -> Self {
        self.refresh_credentials = None;
        self
    }

    /// Enable concurrency limiting for API requests.
    ///
    /// Xero enforces a limit of 5 concurrent requests per organization.
    /// This method adds a semaphore-based limiter to prevent exceeding this limit.
    ///
    /// # Arguments
    ///
    /// * `max_concurrent` - Maximum number of concurrent requests allowed (recommend 5 for Xero)
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use xero_rs::{Client, KeyPair};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let key_pair = KeyPair::from_env();
    /// let client = Client::from_client_credentials(key_pair.clone(), None)
    ///     .await?
    ///     .with_auto_refresh(key_pair)
    ///     .with_concurrency_limit(5);  // Xero's concurrent request limit
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn with_concurrency_limit(mut self, max_concurrent: usize) -> Self {
        self.concurrency_limiter = Some(Arc::new(tokio::sync::Semaphore::new(max_concurrent)));
        self
    }

    /// Disable concurrency limiting.
    #[must_use]
    pub fn without_concurrency_limit(mut self) -> Self {
        self.concurrency_limiter = None;
        self
    }

    /// Build a request object with authentication headers.
    pub(crate) async fn build_request<U: IntoUrl + fmt::Debug>(
        &self,
        method: Method,
        url: U,
    ) -> RequestBuilder {
        self.build_http_client()
            .await
            .request(method, url)
            .header(header::ACCEPT, "application/json")
    }

    /// Get the current rate limit information
    pub async fn rate_limit_info(&self) -> RateLimitInfo {
        self.token_state.read().await.rate_limit_info.clone()
    }

    /// Clear the access token for testing purposes
    ///
    /// # Warning
    /// This is intended for testing only and will invalidate the current access token.
    #[doc(hidden)]
    pub async fn clear_access_token_for_testing(&self) {
        let mut token_state = self.token_state.write().await;
        token_state.access_token = AccessToken::new("invalid_token".to_string());
    }

    /// Execute a GET request with automatic retry for rate limit errors and token expiry
    async fn execute_get<T, Q>(&self, url: Url, query: &Q) -> Result<T>
    where
        T: DeserializeOwned,
        Q: Serialize,
    {
        let mut attempts = 0;
        let mut token_refreshed = false;

        loop {
            // Build and execute the request
            let response = self
                .build_request(Method::GET, url.clone())
                .await
                .query(query)
                .send()
                .await;

            match response {
                Ok(response) => {
                    self.update_rate_limit_info(response.headers()).await;

                    match Self::handle_response(response).await {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            // Check for token expiry
                            if let Error::API(ref api_err) = e
                                && !token_refreshed
                                && matches!(api_err.error, error::ErrorType::UnauthorisedException)
                            {
                                // Check if we have refresh credentials or token
                                let has_refresh_capability = self.refresh_credentials.is_some()
                                    || self.token_state.read().await.refresh_token.is_some();

                                if has_refresh_capability && self.refresh_credentials.is_some() {
                                    tracing::debug!("Token expired, attempting automatic refresh");
                                    let key_pair = self.refresh_credentials.clone().unwrap();

                                    // Attempt to refresh the token
                                    match self.refresh_access_token(key_pair).await {
                                        Ok(()) => {
                                            tracing::info!("Successfully refreshed access token");
                                            token_refreshed = true;
                                            // Retry the request with the new token
                                            continue;
                                        }
                                        Err(refresh_err) => {
                                            tracing::error!(
                                                "Failed to refresh access token: {:?}",
                                                refresh_err
                                            );
                                            // Return the original unauthorized error if refresh fails
                                            return Err(e);
                                        }
                                    }
                                }
                            }
                            // Check for rate limiting
                            if let Error::RateLimitExceeded { retry_after, .. } = e
                                && attempts < MAX_RETRY_ATTEMPTS
                            {
                                attempts += 1;
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
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Execute a POST request with automatic retry for rate limit errors and token expiry
    async fn execute_post<T, B>(&self, url: Url, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        let mut attempts = 0;
        let mut token_refreshed = false;

        // Log the request payload for debugging
        if let Ok(json_body) = serde_json::to_string_pretty(body) {
            tracing::debug!(
                url = %url,
                request_body = %json_body,
                "POST request payload"
            );
        }

        loop {
            // Build and execute the request
            let response = self
                .build_request(Method::POST, url.clone())
                .await
                .json(body)
                .send()
                .await;

            match response {
                Ok(response) => {
                    self.update_rate_limit_info(response.headers()).await;

                    match Self::handle_response(response).await {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            // Check for token expiry
                            if let Error::API(ref api_err) = e
                                && !token_refreshed
                                && matches!(api_err.error, error::ErrorType::UnauthorisedException)
                            {
                                // Check if we have refresh credentials or token
                                let has_refresh_capability = self.refresh_credentials.is_some()
                                    || self.token_state.read().await.refresh_token.is_some();

                                if has_refresh_capability && self.refresh_credentials.is_some() {
                                    tracing::debug!("Token expired, attempting automatic refresh");
                                    let key_pair = self.refresh_credentials.clone().unwrap();

                                    // Attempt to refresh the token
                                    match self.refresh_access_token(key_pair).await {
                                        Ok(()) => {
                                            tracing::info!("Successfully refreshed access token");
                                            token_refreshed = true;
                                            // Retry the request with the new token
                                            continue;
                                        }
                                        Err(refresh_err) => {
                                            tracing::error!(
                                                "Failed to refresh access token: {:?}",
                                                refresh_err
                                            );
                                            // Return the original unauthorized error if refresh fails
                                            return Err(e);
                                        }
                                    }
                                }
                            }
                            // Check for rate limiting
                            if let Error::RateLimitExceeded { retry_after, .. } = e
                                && attempts < MAX_RETRY_ATTEMPTS
                            {
                                attempts += 1;
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
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Execute a PUT request with automatic retry for rate limit errors and token expiry
    async fn execute_put<T, B>(&self, url: Url, body: &B) -> Result<T>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        let mut attempts = 0;
        let mut token_refreshed = false;

        loop {
            // Build and execute the request
            let response = self
                .build_request(Method::PUT, url.clone())
                .await
                .json(body)
                .send()
                .await;

            match response {
                Ok(response) => {
                    self.update_rate_limit_info(response.headers()).await;

                    match Self::handle_response(response).await {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            // Check for token expiry
                            if let Error::API(ref api_err) = e
                                && !token_refreshed
                                && matches!(api_err.error, error::ErrorType::UnauthorisedException)
                            {
                                // Check if we have refresh credentials or token
                                let has_refresh_capability = self.refresh_credentials.is_some()
                                    || self.token_state.read().await.refresh_token.is_some();

                                if has_refresh_capability && self.refresh_credentials.is_some() {
                                    tracing::debug!("Token expired, attempting automatic refresh");
                                    let key_pair = self.refresh_credentials.clone().unwrap();

                                    // Attempt to refresh the token
                                    match self.refresh_access_token(key_pair).await {
                                        Ok(()) => {
                                            tracing::info!("Successfully refreshed access token");
                                            token_refreshed = true;
                                            // Retry the request with the new token
                                            continue;
                                        }
                                        Err(refresh_err) => {
                                            tracing::error!(
                                                "Failed to refresh access token: {:?}",
                                                refresh_err
                                            );
                                            // Return the original unauthorized error if refresh fails
                                            return Err(e);
                                        }
                                    }
                                }
                            }
                            // Check for rate limiting
                            if let Error::RateLimitExceeded { retry_after, .. } = e
                                && attempts < MAX_RETRY_ATTEMPTS
                            {
                                attempts += 1;
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
                            }
                            return Err(e);
                        }
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Execute a DELETE request with automatic retry for rate limit errors and token expiry
    async fn execute_delete(&self, url: Url) -> Result<()> {
        let mut attempts = 0;
        let mut token_refreshed = false;

        loop {
            // Build and execute the request
            let response = self
                .build_request(Method::DELETE, url.clone())
                .await
                .send()
                .await;

            match response {
                Ok(response) => {
                    self.update_rate_limit_info(response.headers()).await;

                    let status = response.status();

                    // Special handling for DELETE responses
                    if status == StatusCode::NO_CONTENT || status == StatusCode::OK {
                        return Ok(());
                    }

                    // Check for rate limiting BEFORE other error handling
                    if status == StatusCode::TOO_MANY_REQUESTS {
                        let retry_after = response
                            .headers()
                            .get(header::RETRY_AFTER)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(Duration::from_secs);

                        let limit_type = response
                            .headers()
                            .get(HEADER_RATE_LIMIT_PROBLEM)
                            .and_then(|v| v.to_str().ok())
                            .map(error::RateLimitType::from_header)
                            .unwrap_or(error::RateLimitType::Unknown("not specified".to_string()));

                        let text = response.text().await.unwrap_or_default();
                        let url = url.to_string();

                        tracing::warn!(
                            "Rate limit exceeded for {}: limit_type={}, retry_after={:?}",
                            url,
                            limit_type,
                            retry_after
                        );

                        return Err(Error::RateLimitExceeded {
                            limit_type,
                            retry_after,
                            status_code: status,
                            url,
                            response_body: Some(text),
                        });
                    }

                    // Try to get error details
                    let content_length = response.content_length().unwrap_or(0);
                    let error = if content_length == 0 {
                        Error::Request(response.error_for_status().unwrap_err())
                    } else {
                        match response.json::<error::Response>().await {
                            Ok(api_error) => Error::API(api_error),
                            Err(e) => Error::Request(e),
                        }
                    };

                    // Check for token expiry
                    if let Error::API(ref api_err) = error
                        && !token_refreshed
                        && matches!(api_err.error, error::ErrorType::UnauthorisedException)
                    {
                        // Check if we have refresh credentials or token
                        let has_refresh_capability = self.refresh_credentials.is_some()
                            || self.token_state.read().await.refresh_token.is_some();

                        if has_refresh_capability && self.refresh_credentials.is_some() {
                            tracing::debug!("Token expired, attempting automatic refresh");
                            let key_pair = self.refresh_credentials.clone().unwrap();

                            // Attempt to refresh the token
                            match self.refresh_access_token(key_pair).await {
                                Ok(()) => {
                                    tracing::info!("Successfully refreshed access token");
                                    token_refreshed = true;
                                    // Retry the request with the new token
                                    continue;
                                }
                                Err(refresh_err) => {
                                    tracing::error!(
                                        "Failed to refresh access token: {:?}",
                                        refresh_err
                                    );
                                    // Return the original unauthorized error if refresh fails
                                    return Err(error);
                                }
                            }
                        }
                    }
                    // Check for rate limiting
                    if let Error::RateLimitExceeded { retry_after, .. } = error
                        && attempts < MAX_RETRY_ATTEMPTS
                    {
                        attempts += 1;
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
                    }
                    return Err(error);
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Perform an authenticated `GET` request against the API with automatic retry.
    #[instrument(skip(self, query))]
    pub async fn get<
        'a,
        R: DeserializeOwned,
        U: AsRef<str> + fmt::Debug + Clone,
        T: Serialize + Sized + fmt::Debug,
    >(
        &self,
        url: U,
        query: &T,
    ) -> Result<R> {
        trace!(?query, ?url, "making GET request");

        // Handle relative URLs by prepending the base URL if needed
        let url_str = url.as_ref();
        let resolved_url = if url_str.starts_with("http://") || url_str.starts_with("https://") {
            // It's already an absolute URL
            Url::parse(url_str).map_err(|_| Error::InvalidEndpoint)?
        } else {
            // It's a relative URL, prepend the base URL
            let base = Url::parse(BASE_URL).map_err(|_| Error::InvalidEndpoint)?;
            base.join(url_str).map_err(|_| Error::InvalidEndpoint)?
        };

        self.execute_get(resolved_url, query).await
    }

    /// Perform a `GET` request against the API using a typed `XeroEndpoint` with automatic retry.
    #[instrument(skip(self, query))]
    pub async fn get_endpoint<'a, R: DeserializeOwned, T: Serialize + Sized + fmt::Debug>(
        &self,
        endpoint: XeroEndpoint,
        query: &T,
    ) -> Result<R> {
        trace!(?query, endpoint = ?endpoint, "making GET request with endpoint");
        let url = endpoint.to_url()?;
        self.execute_get(url, query).await
    }

    /// Perform an authenticated `PUT` request against the API with automatic retry.
    #[instrument(skip(self, data))]
    pub async fn put<
        'a,
        R: DeserializeOwned,
        U: AsRef<str> + fmt::Debug + Clone,
        T: Serialize + Sized,
    >(
        &self,
        url: U,
        data: &T,
    ) -> Result<R> {
        trace!(json = ?serde_json::to_string(data).unwrap(), ?url, "making PUT request");

        // Handle relative URLs by prepending the base URL if needed
        let url_str = url.as_ref();
        let resolved_url = if url_str.starts_with("http://") || url_str.starts_with("https://") {
            // It's already an absolute URL
            Url::parse(url_str).map_err(|_| Error::InvalidEndpoint)?
        } else {
            // It's a relative URL, prepend the base URL
            let base = Url::parse(BASE_URL).map_err(|_| Error::InvalidEndpoint)?;
            base.join(url_str).map_err(|_| Error::InvalidEndpoint)?
        };

        self.execute_put(resolved_url, data).await
    }

    /// Perform an authenticated `POST` request against the API with automatic retry.
    #[instrument(skip(self, data))]
    pub async fn post<
        'a,
        R: DeserializeOwned,
        U: AsRef<str> + fmt::Debug + Clone,
        T: Serialize + Sized + fmt::Debug,
    >(
        &self,
        url: U,
        data: &T,
    ) -> Result<R> {
        trace!(json = ?serde_json::to_string(data).unwrap(), ?url, "making POST request");

        // Handle relative URLs by prepending the base URL if needed
        let url_str = url.as_ref();
        let resolved_url = if url_str.starts_with("http://") || url_str.starts_with("https://") {
            // It's already an absolute URL
            Url::parse(url_str).map_err(|_| Error::InvalidEndpoint)?
        } else {
            // It's a relative URL, prepend the base URL
            let base = Url::parse(BASE_URL).map_err(|_| Error::InvalidEndpoint)?;
            base.join(url_str).map_err(|_| Error::InvalidEndpoint)?
        };

        self.execute_post(resolved_url, data).await
    }

    /// Perform a `POST` request against the API using a typed `XeroEndpoint` with automatic retry.
    #[instrument(skip(self, data))]
    pub async fn post_endpoint<'a, R: DeserializeOwned, T: Serialize + Sized + fmt::Debug>(
        &self,
        endpoint: XeroEndpoint,
        data: &T,
    ) -> Result<R> {
        trace!(json = ?serde_json::to_string(data).unwrap(), endpoint = ?endpoint, "making POST request with endpoint");
        let url = endpoint.to_url()?;
        self.execute_post(url, data).await
    }

    /// Perform a `PUT` request against the API using a typed `XeroEndpoint` with automatic retry.
    #[instrument(skip(self, data))]
    pub async fn put_endpoint<'a, R: DeserializeOwned, T: Serialize + Sized>(
        &self,
        endpoint: XeroEndpoint,
        data: &T,
    ) -> Result<R> {
        trace!(json = ?serde_json::to_string(data).unwrap(), endpoint = ?endpoint, "making PUT request with endpoint");
        let url = endpoint.to_url()?;
        self.execute_put(url, data).await
    }

    /// Perform an authenticated `DELETE` request against the API with automatic retry.
    #[instrument(skip(self))]
    pub async fn delete<U: AsRef<str> + fmt::Debug + Clone>(&self, url: U) -> Result<()> {
        trace!(?url, "making DELETE request");

        // Handle relative URLs by prepending the base URL if needed
        let url_str = url.as_ref();
        let resolved_url = if url_str.starts_with("http://") || url_str.starts_with("https://") {
            // It's already an absolute URL
            Url::parse(url_str).map_err(|_| Error::InvalidEndpoint)?
        } else {
            // It's a relative URL, prepend the base URL
            let base = Url::parse(BASE_URL).map_err(|_| Error::InvalidEndpoint)?;
            base.join(url_str).map_err(|_| Error::InvalidEndpoint)?
        };

        self.execute_delete(resolved_url).await
    }

    /// Perform a `DELETE` request against the API using a typed `XeroEndpoint` with automatic retry.
    #[instrument(skip(self))]
    pub async fn delete_endpoint(&self, endpoint: XeroEndpoint) -> Result<()> {
        trace!(endpoint = ?endpoint, "making DELETE request with endpoint");
        let url = endpoint.to_url()?;
        self.execute_delete(url).await
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
            let limit_type = response
                .headers()
                .get(HEADER_RATE_LIMIT_PROBLEM)
                .and_then(|v| v.to_str().ok())
                .map(error::RateLimitType::from_header)
                .unwrap_or(error::RateLimitType::Unknown("not specified".to_string()));

            let retry_after = response
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(std::time::Duration::from_secs);

            // Log rate limit hit with detailed information
            tracing::warn!(
                "Rate limit exceeded for {}: limit_type={}, retry_after={:?}, day_remaining={:?}, minute_remaining={:?}, app_minute_remaining={:?}",
                url,
                limit_type,
                retry_after,
                rate_limit_info.day_limit_remaining,
                rate_limit_info.minute_limit_remaining,
                rate_limit_info.app_minute_limit_remaining
            );

            // Get response text for error context
            let text = response.text().await.unwrap_or_default();

            return Err(Error::RateLimitExceeded {
                limit_type,
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
                tracing::error!(
                    "Deserialization error: {}, near position: {} - response text around that position: {}",
                    e,
                    e.column(),
                    &text
                        .chars()
                        .skip(e.column().saturating_sub(30))
                        .take(100)
                        .collect::<String>()
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
            StatusCode::UNAUTHORIZED => {
                let mut body = serde_json::from_str::<serde_json::Value>(&text).unwrap_or_default();
                body["Type"] = "UnauthorisedException".into();
                // Try to parse as UnauthorisedException
                match serde_json::from_value::<error::Response>(body) {
                    Ok(api_error)
                        if matches!(api_error.error, error::ErrorType::UnauthorisedException) =>
                    {
                        tracing::debug!("Received 401 Unauthorized response");
                        Err(Error::API(api_error))
                    }
                    Ok(api_error) => {
                        // It's some other API error on a 401 response
                        Err(Error::API(api_error))
                    }
                    Err(e) => {
                        // Couldn't parse the error response, return as deserialization error
                        tracing::error!(
                            "Failed to parse 401 response: {}, raw response: {}",
                            e,
                            text
                        );
                        Err(handle_deserialize_error(e))
                    }
                }
            }
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
                StatusCode::BAD_REQUEST => {
                    // Try to parse as API error response
                    match serde_json::from_str::<error::Response>(&text) {
                        Ok(api_error) => {
                            tracing::error!(
                                url = %url,
                                status = %status,
                                error_number = ?api_error.error_number,
                                message = ?api_error.message,
                                detail = ?api_error.detail,
                                error_type = ?std::mem::discriminant(&api_error.error),
                                raw_response = %text,
                                "API error response from Xero"
                            );

                            // Additional logging for ValidationException
                            if let error::ErrorType::ValidationException {
                                ref elements,
                                ref timesheets,
                            } = api_error.error
                            {
                                tracing::warn!(
                                    elements_count = elements.len(),
                                    has_timesheets = timesheets.is_some(),
                                    "ValidationException details: {} validation elements",
                                    elements.len()
                                );

                                // Note: Since we removed #[serde(default)], reaching this code with
                                // empty elements means Xero actually sent Elements: [] in the response.
                                // This is unusual but not necessarily a deserialization error.
                                if elements.is_empty() {
                                    tracing::warn!(
                                        "Unusual: Xero returned ValidationException with empty Elements array. Raw response: {}",
                                        text
                                    );
                                }
                            }

                            Err(Error::API(api_error))
                        }
                        Err(e) => {
                            // If we can't parse the error response, include the raw text
                            tracing::error!(
                                "Failed to parse API error response: {}, raw response: {}",
                                e,
                                text
                            );
                            // Try to extract basic error info from the raw response
                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&text)
                            {
                                let error_type = json_value
                                    .get("Type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown");
                                let message = json_value
                                    .get("Message")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&text);
                                let error_number = json_value
                                    .get("ErrorNumber")
                                    .and_then(serde_json::Value::as_u64)
                                    .unwrap_or(0);

                                tracing::error!(
                                    "Xero API error: Type={}, ErrorNumber={}, Message={}",
                                    error_type,
                                    error_number,
                                    message
                                );
                            }
                            Err(handle_deserialize_error(e))
                        }
                    }
                }
                _ => {
                    tracing::error!("Unexpected status code: {}", status);
                    // Try to parse as API error first
                    match serde_json::from_str::<error::Response>(&text) {
                        Ok(api_error) => Err(Error::API(api_error)),
                        Err(e) => {
                            // If it's not an API error, return generic error with details
                            tracing::error!(
                                "Failed to parse response as API error: {}, raw response: {}",
                                e,
                                text
                            );
                            Err(handle_deserialize_error(e))
                        }
                    }
                }
            },
        }
    }

    /// Access the accounts API (chart of accounts)
    #[must_use]
    pub fn accounts(&self) -> AccountsApi<'_> {
        AccountsApi { client: self }
    }

    /// Access the contacts API
    #[must_use]
    pub fn contacts(&self) -> ContactsApi<'_> {
        ContactsApi { client: self }
    }

    /// Access the invoices API
    #[must_use]
    pub fn invoices(&self) -> InvoicesApi<'_> {
        InvoicesApi { client: self }
    }

    /// Access the purchase orders API
    #[must_use]
    pub fn purchase_orders(&self) -> PurchaseOrdersApi<'_> {
        PurchaseOrdersApi { client: self }
    }

    /// Access the quotes API
    #[must_use]
    pub fn quotes(&self) -> QuotesApi<'_> {
        QuotesApi { client: self }
    }

    /// Access the timesheets API
    #[must_use]
    pub fn timesheets(&self) -> TimesheetsApi<'_> {
        TimesheetsApi { client: self }
    }

    /// Access the employees API
    #[must_use]
    pub fn employees(&self) -> EmployeesApi<'_> {
        EmployeesApi { client: self }
    }

    /// Access the earnings rates API
    #[must_use]
    pub fn earnings_rates(&self) -> EarningsRatesApi<'_> {
        EarningsRatesApi { client: self }
    }

    /// Access the pay calendars API
    #[must_use]
    pub fn pay_calendars(&self) -> PayCalendarsApi<'_> {
        PayCalendarsApi { client: self }
    }

    /// Access the items API
    #[must_use]
    pub fn items(&self) -> ItemsApi<'_> {
        ItemsApi { client: self }
    }

    /// Access the leave applications API
    #[must_use]
    pub fn leave_applications(&self) -> LeaveApplicationsApi<'_> {
        LeaveApplicationsApi { client: self }
    }

    /// Access the leave types API
    #[must_use]
    pub fn leave_types(&self) -> LeaveTypesApi<'_> {
        LeaveTypesApi { client: self }
    }
}

/// API handler for Accounts (Chart of Accounts) endpoints
#[derive(Debug)]
pub struct AccountsApi<'a> {
    client: &'a Client,
}

impl AccountsApi<'_> {
    /// Retrieve a list of accounts with optional filtering
    #[instrument(skip(self, parameters))]
    pub async fn list(&self, parameters: account::ListParameters) -> Result<Vec<Account>> {
        account::list(self.client, parameters).await
    }

    /// List all accounts without any filtering
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> Result<Vec<Account>> {
        account::list_all(self.client).await
    }

    /// Retrieve a single account by ID
    #[instrument(skip(self))]
    pub async fn get(&self, account_id: Uuid) -> Result<Account> {
        account::get(self.client, account_id).await
    }

    /// Create a new account
    #[instrument(skip(self, account))]
    pub async fn create(&self, account: &account::Builder) -> Result<Account> {
        account::create(self.client, account).await
    }

    /// Update an existing account
    #[instrument(skip(self, account))]
    pub async fn update(&self, account_id: Uuid, account: &account::Builder) -> Result<Account> {
        account::update(self.client, account_id, account).await
    }

    /// Delete an account
    #[instrument(skip(self))]
    pub async fn delete(&self, account_id: Uuid) -> Result<()> {
        account::delete(self.client, account_id).await
    }

    /// List all attachments for an account
    #[instrument(skip(self))]
    pub async fn list_attachments(&self, account_id: Uuid) -> Result<Vec<account::Attachment>> {
        account::list_attachments(self.client, account_id).await
    }

    /// Get a specific attachment by ID
    #[instrument(skip(self))]
    pub async fn get_attachment(&self, account_id: Uuid, attachment_id: Uuid) -> Result<Vec<u8>> {
        account::get_attachment(self.client, account_id, attachment_id).await
    }

    /// Upload an attachment to an account
    #[instrument(skip(self, attachment_content))]
    pub async fn upload_attachment(
        &self,
        account_id: Uuid,
        filename: &str,
        attachment_content: &[u8],
    ) -> Result<account::Attachment> {
        account::upload_attachment(self.client, account_id, filename, attachment_content).await
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
        let response: contact::ListResponse = self
            .client
            .get_endpoint(XeroEndpoint::Contacts, &empty_vec)
            .await?;
        Ok(response.contacts)
    }

    /// Retrieve a single contact by ID
    #[instrument(skip(self))]
    pub async fn get(&self, contact_id: Uuid) -> Result<Contact> {
        let endpoint = XeroEndpoint::Contact(contact_id);
        let empty_vec: Vec<String> = Vec::new();
        let response: contact::ListResponse = self
            .client
            .get_endpoint(endpoint.clone(), &empty_vec)
            .await?;
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
        let response: invoice::ListResponse = self
            .client
            .get_endpoint(XeroEndpoint::Invoices, &parameters)
            .await?;
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
        let response: invoice::ListResponse = self
            .client
            .get_endpoint(endpoint.clone(), &empty_tuple)
            .await?;
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

        let response: MutationResponse = self
            .client
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
    pub async fn update(
        &mut self,
        invoice_id: Uuid,
        invoice: &invoice::Builder,
    ) -> Result<Invoice> {
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
    pub async fn create_history(
        &self,
        invoice_id: Uuid,
        details: &str,
    ) -> Result<Vec<invoice::HistoryRecord>> {
        invoice::create_history(self.client, invoice_id, details).await
    }

    /// List attachments for an invoice
    #[instrument(skip(self))]
    pub async fn list_attachments(&self, invoice_id: Uuid) -> Result<Vec<invoice::Attachment>> {
        invoice::list_attachments(self.client, invoice_id).await
    }

    /// Get a specific attachment by ID
    #[instrument(skip(self))]
    pub async fn get_attachment(
        &mut self,
        invoice_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Vec<u8>> {
        invoice::get_attachment(self.client, invoice_id, attachment_id).await
    }

    /// Get an attachment by filename
    #[instrument(skip(self))]
    pub async fn get_attachment_by_filename(
        &mut self,
        invoice_id: Uuid,
        filename: &str,
    ) -> Result<Vec<u8>> {
        invoice::get_attachment_by_filename(self.client, invoice_id, filename).await
    }

    /// Upload an attachment to an invoice
    #[instrument(skip(self, attachment_content))]
    pub async fn upload_attachment(
        &mut self,
        invoice_id: Uuid,
        filename: &str,
        attachment_content: &[u8],
    ) -> Result<invoice::Attachment> {
        invoice::upload_attachment(self.client, invoice_id, filename, attachment_content).await
    }

    /// Update an existing attachment
    #[instrument(skip(self, attachment_content))]
    pub async fn update_attachment(
        &mut self,
        invoice_id: Uuid,
        filename: &str,
        attachment_content: &[u8],
    ) -> Result<invoice::Attachment> {
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
        let response: purchase_order::ListResponse = self
            .client
            .get(purchase_order::ENDPOINT, &empty_vec)
            .await?;
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
        let request = purchase_order::PurchaseOrdersRequest::single(builder);
        let result: MutationResponse = self.client.put(purchase_order::ENDPOINT, &request).await?;
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

    /// Update an existing purchase order
    #[instrument(skip(self, builder))]
    pub async fn update(
        &self,
        purchase_order_id: Uuid,
        builder: &purchase_order::Builder,
    ) -> Result<PurchaseOrder> {
        let endpoint = format!("{}{}", purchase_order::ENDPOINT, purchase_order_id);
        let request = purchase_order::PurchaseOrdersRequest::single(builder);
        let result: MutationResponse = self.client.post(&endpoint, &request).await?;
        result
            .data
            .get_purchase_orders()
            .and_then(|po| po.into_iter().next())
            .ok_or(Error::NotFound {
                entity: "PurchaseOrder".to_string(),
                url: endpoint,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(
                    "Failed to update purchase order - no purchase order in response".to_string(),
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
    pub async fn create_history(
        &self,
        quote_id: Uuid,
        details: &str,
    ) -> Result<Vec<quote::HistoryRecord>> {
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
    pub async fn get_attachment_by_filename(
        &self,
        quote_id: Uuid,
        filename: &str,
    ) -> Result<Vec<u8>> {
        quote::get_attachment_by_filename(self.client, quote_id, filename).await
    }

    /// Upload an attachment to a quote
    #[instrument(skip(self, attachment_content))]
    pub async fn upload_attachment(
        &mut self,
        quote_id: Uuid,
        filename: &str,
        attachment_content: &[u8],
    ) -> Result<quote::Attachment> {
        quote::upload_attachment(self.client, quote_id, filename, attachment_content).await
    }

    /// Update an existing attachment
    #[instrument(skip(self, attachment_content))]
    pub async fn update_attachment(
        &mut self,
        quote_id: Uuid,
        filename: &str,
        attachment_content: &[u8],
    ) -> Result<quote::Attachment> {
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
        modified_after: Option<String>,
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
        let response: employee::ListResponse =
            self.client.get(employee::ENDPOINT, &empty_vec).await?;
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
        let response: ListResponse = self
            .client
            .get(earnings_rates::ENDPOINT, &empty_vec)
            .await?;
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
        let url =
            format!("https://api.xero.com/payroll.xro/1.0/PayrollCalendars/{pay_calendar_id}");
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
    pub async fn create(
        &mut self,
        pay_calendar: &pay_calendar::CreatePayCalendar,
    ) -> Result<PayCalendar> {
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

/// API handler for Items endpoints
#[derive(Debug)]
pub struct ItemsApi<'a> {
    client: &'a Client,
}

impl ItemsApi<'_> {
    /// Retrieve a list of items with optional filtering
    #[instrument(skip(self, parameters))]
    pub async fn list(&self, parameters: item::ListParameters) -> Result<Vec<Item>> {
        item::list(self.client, parameters).await
    }

    /// List all items without any filtering
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> Result<Vec<Item>> {
        item::list_all(self.client).await
    }

    /// Retrieve a single item by ID
    #[instrument(skip(self))]
    pub async fn get(&self, item_id: Uuid) -> Result<Item> {
        item::get(self.client, item_id).await
    }

    /// Retrieve a single item by code
    #[instrument(skip(self))]
    pub async fn get_by_code(&self, code: &str) -> Result<Item> {
        item::get_by_code(self.client, code).await
    }

    /// Create a single item
    #[instrument(skip(self, item))]
    pub async fn create(&self, item: &item::Builder) -> Result<Item> {
        item::create_single(self.client, item).await
    }

    /// Create multiple items
    #[instrument(skip(self, items))]
    pub async fn create_multiple(&self, items: &[item::Builder]) -> Result<Vec<Item>> {
        item::create(self.client, items).await
    }

    /// Update or create a single item
    #[instrument(skip(self, item))]
    pub async fn update_or_create(&self, item: &item::Builder) -> Result<Item> {
        let items = item::update_or_create(self.client, &[item.clone()]).await?;
        items.into_iter().next().ok_or(Error::NotFound {
            entity: "Item".to_string(),
            url: item::ENDPOINT.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some("No item returned in response".to_string()),
        })
    }

    /// Update or create multiple items
    #[instrument(skip(self, items))]
    pub async fn update_or_create_multiple(
        &mut self,
        items: &[item::Builder],
    ) -> Result<Vec<Item>> {
        item::update_or_create(self.client, items).await
    }

    /// Update a specific item
    #[instrument(skip(self, item))]
    pub async fn update(&self, item_id: Uuid, item: &item::Builder) -> Result<Item> {
        item::update(self.client, item_id, item).await
    }

    /// Delete a specific item
    #[instrument(skip(self))]
    pub async fn delete(&self, item_id: Uuid) -> Result<()> {
        item::delete(self.client, item_id).await
    }

    /// Get the history for an item
    #[instrument(skip(self))]
    pub async fn get_history(&self, item_id: Uuid) -> Result<Vec<item::HistoryRecord>> {
        item::get_history(self.client, item_id).await
    }

    /// Create a history record for an item
    #[instrument(skip(self))]
    pub async fn create_history(
        &self,
        item_id: Uuid,
        details: &str,
    ) -> Result<Vec<item::HistoryRecord>> {
        item::create_history(self.client, item_id, details).await
    }
}

/// API handler for Leave Applications endpoints
#[derive(Debug)]
pub struct LeaveApplicationsApi<'a> {
    client: &'a Client,
}

impl LeaveApplicationsApi<'_> {
    /// List approved leave applications (v1 endpoint)
    ///
    /// This endpoint only returns leave applications that have been approved.
    /// Use `list_v2` to get all leave including pending and rejected.
    ///
    /// # Parameters
    ///
    /// * `parameters` - Optional filter parameters
    /// * `modified_after` - Optional ISO8601 timestamp (format: yyyy-mm-ddThh:mm:ss) to filter by modification date
    #[instrument(skip(self, parameters, modified_after))]
    pub async fn list(
        &self,
        parameters: Option<leave_application::ListParameters>,
        modified_after: Option<String>,
    ) -> Result<Vec<LeaveApplication>> {
        LeaveApplication::list(self.client, parameters.as_ref(), modified_after).await
    }

    /// List all leave applications (v2 endpoint)
    ///
    /// This endpoint returns leave with all statuses: SCHEDULED, PROCESSED,
    /// REQUESTED (awaiting approval), and REJECTED.
    ///
    /// # Parameters
    ///
    /// * `parameters` - Optional filter parameters
    /// * `modified_after` - Optional ISO8601 timestamp to filter by modification date
    #[instrument(skip(self, parameters, modified_after))]
    pub async fn list_v2(
        &self,
        parameters: Option<leave_application::ListParameters>,
        modified_after: Option<String>,
    ) -> Result<Vec<LeaveApplication>> {
        LeaveApplication::list_v2(self.client, parameters.as_ref(), modified_after).await
    }

    /// List all approved leave without filtering
    #[instrument(skip(self))]
    pub async fn list_all(&self) -> Result<Vec<LeaveApplication>> {
        self.list(None, None).await
    }

    /// Retrieve a single leave application by ID
    #[instrument(skip(self))]
    pub async fn get(&self, leave_application_id: Uuid) -> Result<LeaveApplication> {
        LeaveApplication::get(self.client, leave_application_id).await
    }

    /// Create a new leave application
    #[instrument(skip(self, leave_application))]
    pub async fn create(
        &self,
        leave_application: &PostLeaveApplication,
    ) -> Result<LeaveApplication> {
        LeaveApplication::post(self.client, leave_application).await
    }

    /// Update an existing leave application
    #[instrument(skip(self, leave_application))]
    pub async fn update(&self, leave_application: &LeaveApplication) -> Result<LeaveApplication> {
        LeaveApplication::update(self.client, leave_application).await
    }

    /// Approve a leave application that is in REQUESTED status
    #[instrument(skip(self))]
    pub async fn approve(&self, leave_application_id: Uuid) -> Result<LeaveApplication> {
        LeaveApplication::approve(self.client, leave_application_id).await
    }

    /// Reject a leave application that is in REQUESTED status
    #[instrument(skip(self))]
    pub async fn reject(&self, leave_application_id: Uuid) -> Result<LeaveApplication> {
        LeaveApplication::reject(self.client, leave_application_id).await
    }
}

/// API handler for Leave Types endpoints
#[derive(Debug)]
pub struct LeaveTypesApi<'a> {
    client: &'a Client,
}

impl LeaveTypesApi<'_> {
    /// Retrieve a list of leave types
    ///
    /// Leave types are retrieved from the PayItems endpoint.
    #[instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<LeaveType>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct PayItems {
            #[serde(default)]
            leave_types: Vec<LeaveType>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct ListResponse {
            pay_items: PayItems,
        }

        let empty_vec: Vec<String> = Vec::new();
        let response: ListResponse = self
            .client
            .get(earnings_rates::ENDPOINT, &empty_vec)
            .await?;
        Ok(response.pay_items.leave_types)
    }
}
