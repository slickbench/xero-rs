use core::fmt;

use oauth2::TokenResponse;
use reqwest::{header, IntoUrl};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

use crate::error::{self, Result};
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
        oauth2::RequestTokenError<oauth2::reqwest::Error<reqwest::Error>, error::Response>,
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

    /// Perform a `GET` request against the API.
    #[instrument(skip(self, query))]
    pub async fn get<'a, R: DeserializeOwned, U: IntoUrl + fmt::Debug, T: Serialize + Sized>(
        &self,
        endpoint: U,
        query: T,
    ) -> Result<R> {
        Ok(self
            .http_client
            .get(endpoint)
            .header(header::ACCEPT, "application/json")
            .query(&query)
            .send()
            .await?
            .json()
            .await?)
    }
}
