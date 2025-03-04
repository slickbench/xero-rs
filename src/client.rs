use core::fmt;
use std::borrow::Cow;

use oauth2::{
    AccessToken, AuthorizationCode, CsrfToken, HttpClientError, RefreshToken, TokenResponse,
};
use reqwest::{header, IntoUrl, Method, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;
use uuid::Uuid;

use crate::error::{self, Error, Result};
use crate::oauth::{KeyPair, OAuthClient};
use crate::scope::Scope;

const XERO_AUTH_URL: &str = "https://login.xero.com/identity/connect/authorize";
const XERO_TOKEN_URL: &str = "https://identity.xero.com/connect/token";

#[allow(unused)]
#[derive(Clone, Debug)]
/// This is the client that is used for interacting with the Xero API. It handles OAuth 2 authentication
/// and context (the current tenant).
pub struct Client {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
    tenant_id: Option<Uuid>,
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
    #[instrument]
    pub fn authorize_url(
        key_pair: KeyPair,
        redirect_url: Url,
        scopes: Vec<Scope>,
    ) -> (Url, CsrfToken) {
        Self::build_oauth_client(key_pair)
            .set_redirect_uri(oauth2::RedirectUrl::from_url(redirect_url))
            .authorize_url(CsrfToken::new_random)
            .add_scopes(scopes.into_iter().map(super::scope::Scope::into_oauth2))
            .url()
    }

    /// # Errors
    /// Returns an error if the connection can't be made.
    #[instrument]
    pub async fn from_client_credentials(
        key_pair: KeyPair,
        scopes: Option<Vec<Scope>>,
    ) -> std::result::Result<
        Self,
        oauth2::RequestTokenError<HttpClientError<reqwest::Error>, error::OAuth2ErrorResponse>,
    > {
        let oauth_client = Self::build_oauth_client(key_pair);
        let http_client = reqwest::Client::new();

        trace!("retrieving access token w/ client credentials grant");
        let token_result = oauth_client
            .exchange_client_credentials()
            .add_scopes(
                scopes
                    .unwrap_or_default()
                    .into_iter()
                    .map(super::scope::Scope::into_oauth2),
            )
            .request_async(&http_client)
            .await?;

        Ok(Self {
            access_token: token_result.access_token().clone(),
            refresh_token: None,
            tenant_id: None,
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

    /// Perform an authenticated `GET` request against the API.
    #[instrument(skip(self, query))]
    pub async fn get<
        'a,
        R: DeserializeOwned,
        U: IntoUrl + fmt::Debug,
        T: Serialize + Sized + fmt::Debug,
    >(
        &self,
        url: U,
        query: T,
    ) -> Result<R> {
        trace!(?query, ?url, "making GET request");
        Self::handle_response(
            self.build_request(Method::GET, url)
                .query(&query)
                .send()
                .await?,
        )
        .await
    }

    /// Perform an authenticated `PUT` request against the API. This method can only create new objects.
    #[instrument(skip(self, data))]
    pub async fn put<'a, R: DeserializeOwned, U: IntoUrl + fmt::Debug, T: Serialize + Sized>(
        &self,
        url: U,
        data: &T,
    ) -> Result<R> {
        trace!(json = ?serde_json::to_string(&data).unwrap(), ?url, "making PUT request");
        Self::handle_response(
            self.build_request(Method::PUT, url)
                .json(data)
                .send()
                .await?,
        )
        .await
    }

    /// Perform an authenticated `POST` request against the API. This method can create or update objects.
    #[instrument(skip(self, data))]
    pub async fn post<
        'a,
        R: DeserializeOwned,
        U: IntoUrl + fmt::Debug,
        T: Serialize + Sized + fmt::Debug,
    >(
        &self,
        url: U,
        data: &T,
    ) -> Result<R> {
        trace!(json = ?serde_json::to_string(&data).unwrap(), ?url, "making POST request");
        Self::handle_response(
            self.build_request(Method::POST, url)
                .json(data)
                .send()
                .await?,
        )
        .await
    }

    /// Perform an authenticated `DELETE` request against the API.
    #[instrument(skip(self))]
    pub async fn delete<U: IntoUrl + fmt::Debug>(&self, url: U) -> Result<()> {
        trace!(?url, "making DELETE request");
        let response = self.build_request(Method::DELETE, url).send().await?;
        if response.status() == StatusCode::NO_CONTENT || response.status() == StatusCode::OK {
            Ok(())
        } else {
            Err(Error::API(response.json().await?))
        }
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

        let text = response.text().await?;
        tracing::debug!("Response body: {}", text);

        let handle_deserialize_error = {
            let text = text.clone();
            |e: serde_json::Error| Error::DeserializationError(e, Some(text))
        };

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
                        tracing::error!("Response body: {}", text);
                        Err(handle_deserialize_error(e))
                    }
                },
                StatusCode::FORBIDDEN => Err(Error::Forbidden(
                    serde_json::from_str(&text).map_err(handle_deserialize_error)?,
                )),
                _ => {
                    tracing::error!("Unexpected status code: {}", status);
                    tracing::error!("Response body: {}", text);
                    Err(Error::API(
                        serde_json::from_str(&text).map_err(handle_deserialize_error)?,
                    ))
                }
            },
        }
    }
}
