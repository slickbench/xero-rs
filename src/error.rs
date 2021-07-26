use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {}

impl oauth2::ErrorResponse for Response {}

/// Errors that can occur when interacting with the Xero API.
#[derive(Debug, Error)]
pub enum Error {
    #[error("error making request: {0:?}")]
    Request(reqwest::Error),

    #[error("error decoding response: {0:?}")]
    DecodeError(serde_json::Error),

    #[error("unexpected response status: {0:?} | body: {1:?}")]
    UnexpectedResponseStatus(StatusCode, Option<String>),

    #[error("object not found")]
    NotFound,

    #[error("endpoint could not be parsed as a URL")]
    InvalidEndpoint,
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Request(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::DecodeError(e)
    }
}

pub type Result<O> = std::result::Result<O, Error>;
