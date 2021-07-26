use core::fmt;

use oauth2::TokenResponse;
use reqwest::{header, IntoUrl, Method, RequestBuilder, StatusCode};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

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
    /// # Errors
    /// Returns an error if the connection can't be made.
    /// # Panics
    #[instrument]
    pub async fn from_client_credentials(
        key_pair: KeyPair,
        scopes: Option<Vec<oauth2::Scope>>,
    ) -> std::result::Result<
        Self,
        oauth2::RequestTokenError<
            oauth2::reqwest::Error<reqwest::Error>,
            error::OAuth2ErrorResponse,
        >,
    > {
        trace!("building oauth2 client");
        let oauth_client: OAuthClient = oauth2::Client::new(
            key_pair.0,
            key_pair.1,
            oauth2::AuthUrl::new(XERO_AUTH_URL.to_string()).unwrap(),
            Some(oauth2::TokenUrl::new(XERO_TOKEN_URL.to_string()).unwrap()),
        );

        trace!("retrieving access token w/ client credentials grant");
        let token_result = oauth_client
            .exchange_client_credentials()
            .add_scopes(scopes.unwrap_or_default())
            .request_async(oauth2::reqwest::async_http_client)
            .await?;
        let access_token = token_result.access_token().secret();

        let mut headers = header::HeaderMap::new();
        headers.append(
            "Authorization",
            header::HeaderValue::from_str(&format!("Bearer {}", access_token)).unwrap(),
        );
        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Ok(Self {
            oauth_client,
            http_client,
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
    pub async fn post<'a, R: DeserializeOwned, U: IntoUrl + fmt::Debug, T: Serialize + Sized>(
        &self,
        url: U,
        data: &T,
    ) -> Result<R> {
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
        {
            Ok(match status {
                StatusCode::OK => Ok(serde_json::from_str(&text)?),
                StatusCode::BAD_REQUEST => Err(Error::XeroError(serde_json::from_str(&text)?)),
                status => Err(match serde_json::from_str(&text) {
                    Err(_) => Error::UnexpectedResponseStatus(status, Some(text.clone())),
                    Ok(error_response) => Error::XeroError(error_response),
                }),
            })
        }
        .map_err(|e| {
            error!(?text, "failed to parse response");
            Error::DeserializationError(e, Some(text))
        })?
    }
}
