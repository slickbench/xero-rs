use core::fmt;
use std::borrow::Cow;

use oauth2::{AuthorizationCode, CsrfToken, TokenResponse};
use reqwest::{header, IntoUrl, Method, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

pub use oauth2::Scope;

use crate::error::{self, Error, Result};
use crate::oauth::{KeyPair, OAuthClient};

const XERO_AUTH_URL: &str = "https://login.xero.com/identity/connect/authorize";
const XERO_TOKEN_URL: &str = "https://identity.xero.com/connect/token";

#[allow(unused)]
pub struct Client {
    oauth_client: OAuthClient,
    http_client: reqwest::Client,
    redirect_uris: Option<Vec<Url>>,
    grant_type: Option<String>,
    scopes: Option<Vec<String>>,
    state: Option<String>,
}

impl Client {
    #[instrument]
    fn build_http_client(access_token: &oauth2::AccessToken) -> reqwest::Client {
        let mut headers = header::HeaderMap::new();
        headers.append(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {}", access_token.secret())).unwrap(),
        );
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap()
    }

    #[instrument]
    fn build_oauth_client(key_pair: KeyPair) -> OAuthClient {
        oauth2::Client::new(
            key_pair.0,
            key_pair.1,
            oauth2::AuthUrl::new(XERO_AUTH_URL.to_string()).unwrap(),
            Some(oauth2::TokenUrl::new(XERO_TOKEN_URL.to_string()).unwrap()),
        )
    }

    /// Generates an authorization URL to use for the code flow authorization method.
    #[instrument]
    pub fn authorize_url(
        key_pair: KeyPair,
        redirect_url: Url,
        scopes: Vec<Scope>,
    ) -> (Url, CsrfToken) {
        let oauth_client = Self::build_oauth_client(key_pair)
            .set_redirect_uri(oauth2::RedirectUrl::from_url(redirect_url));

        oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(scopes)
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
        oauth2::RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            error::OAuth2ErrorResponse,
        >,
    > {
        let oauth_client = Self::build_oauth_client(key_pair);

        trace!("retrieving access token w/ client credentials grant");
        let token_result = oauth_client
            .exchange_client_credentials()
            .add_scopes(scopes.unwrap_or_default())
            .request_async(oauth2::reqwest::async_http_client)
            .await?;

        Ok(Self {
            oauth_client,
            http_client: Self::build_http_client(token_result.access_token()),
            redirect_uris: None,
            grant_type: None,
            scopes: None,
            state: None,
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
        oauth2::RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            error::OAuth2ErrorResponse,
        >,
    > {
        let oauth_client = Self::build_oauth_client(key_pair);
        let token_result = oauth_client
            .exchange_code(AuthorizationCode::new(code))
            .set_redirect_uri(Cow::Owned(oauth2::RedirectUrl::from_url(redirect_url)))
            .request_async(oauth2::reqwest::async_http_client)
            .await?;

        Ok(Self {
            oauth_client,
            http_client: Self::build_http_client(token_result.access_token()),
            redirect_uris: None,
            grant_type: None,
            scopes: None,
            state: None,
        })
    }

    /// Build a request object with authentication headers.
    fn build_request<U: IntoUrl + fmt::Debug>(&self, method: Method, url: U) -> RequestBuilder {
        self.http_client
            .request(method, url)
            .header(header::ACCEPT, "application/json")
    }

    /// Perform a `GET` request against the API.
    #[instrument(skip(self, query))]
    pub async fn get<'a, R: DeserializeOwned, U: IntoUrl + fmt::Debug, T: Serialize + Sized>(
        &self,
        url: U,
        query: T,
    ) -> Result<R> {
        Self::handle_response(
            self.build_request(Method::GET, url)
                .query(&query)
                .send()
                .await?,
        )
        .await
    }

    /// Perform a `PUT` request against the API. This method can only create new objects.
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

    /// Perform a `POST` request against the API. This method can create or update objects.
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

    #[instrument(skip(response))]
    async fn handle_response<T: DeserializeOwned + Sized>(
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();
        let text = response.text().await?;
        let handle_deserialize_error = {
            let text = text.clone();
            |e: serde_json::Error| Error::DeserializationError(e, Some(text))
        };

        match status {
            StatusCode::NOT_FOUND => Err(Error::NotFound),
            status => match status {
                StatusCode::OK => serde_json::from_str(&text).map_err(handle_deserialize_error),
                _ => Err(Error::XeroError(
                    serde_json::from_str(&text).map_err(handle_deserialize_error)?,
                )),
            },
        }
    }
}
