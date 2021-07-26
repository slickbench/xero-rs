use std::time::Duration;

use oauth2::{
    basic::{BasicTokenIntrospectionResponse, BasicTokenType},
    RefreshToken, StandardRevocableToken,
};
use serde::{Deserialize, Serialize};

use crate::error;

/// Stores the OAuth 2 client ID and client secret.
#[derive(Debug, Clone)]
pub struct KeyPair(
    pub(crate) oauth2::ClientId,
    pub(crate) Option<oauth2::ClientSecret>,
);

impl KeyPair {
    /// Creates a new `KeyPair` from the provided `client_id` and `client_secret` strings.
    #[must_use]
    pub fn new(client_id: String, client_secret: Option<String>) -> Self {
        Self(
            oauth2::ClientId::new(client_id),
            client_secret.map(oauth2::ClientSecret::new),
        )
    }

    /// Creates a new `KeyPair` from `XERO_CLIENT_ID` and `XERO_CLIENT_SECRET` environment variables.
    ///
    /// # Panics
    /// Panics if `XERO_CLIENT_ID` environment variable is not set.
    #[must_use]
    pub fn from_env() -> Self {
        Self(
            oauth2::ClientId::new(std::env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID not set")),
            std::env::var("XERO_CLIENT_SECRET")
                .ok()
                .map(oauth2::ClientSecret::new),
        )
    }
}

pub type OAuthClient = oauth2::Client<
    error::OAuth2ErrorResponse,
    TokenResponse,
    BasicTokenType,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    error::OAuth2ErrorResponse,
>;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    access_token: oauth2::AccessToken,
    id_token: Option<String>,
    expires_in: u64,
    token_type: BasicTokenType,
    refresh_token: Option<RefreshToken>,
}

impl oauth2::TokenResponse<BasicTokenType> for TokenResponse {
    fn access_token(&self) -> &oauth2::AccessToken {
        &self.access_token
    }
    fn token_type(&self) -> &BasicTokenType {
        &self.token_type
    }

    fn expires_in(&self) -> Option<std::time::Duration> {
        Some(Duration::from_secs(self.expires_in))
    }

    fn refresh_token(&self) -> Option<&RefreshToken> {
        self.refresh_token.as_ref()
    }

    fn scopes(&self) -> Option<&Vec<oauth2::Scope>> {
        None
    }
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct IdToken {
    nbf: i64,
    exp: i64,
    iss: String,
    aud: String,
    iat: i64,
    at_hash: String,
    sid: String,
    sub: String,
    auth_time: i64,
    idp: String,
    xero_userid: String,
    global_session_id: String,
    preferred_username: String,
    email: String,
    given_name: String,
    family_name: String,
    amr: Vec<String>,
}

#[derive(Deserialize)]
#[allow(unused)]
pub struct AccessToken {
    nbf: i64,
    exp: i64,
    iss: String,
    aud: String,
    client_id: String,
    sub: String,
    auth_time: i64,
    idp: String,
    xero_userid: String,
    global_session_id: String,
    jti: String,
    scope: Vec<String>,
    amr: Vec<String>,
}
