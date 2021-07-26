use std::time::Duration;

use oauth2::{RefreshToken, StandardRevocableToken, basic::{BasicTokenIntrospectionResponse, BasicTokenType}};
use serde::{Serialize, Deserialize};

use crate::error::ErrorResponse;

pub type OAuthClient = oauth2::Client<ErrorResponse, TokenResponse, BasicTokenType, BasicTokenIntrospectionResponse, StandardRevocableToken, ErrorResponse>;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    access_token: oauth2::AccessToken,
    id_token: Option<String>,
    expires_in: u64,
    token_type: BasicTokenType,
    refresh_token: Option<RefreshToken>
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
    amr: Vec<String>
}

#[derive(Deserialize)]
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
    amr: Vec<String>
}

