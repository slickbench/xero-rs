use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {}

impl oauth2::ErrorResponse for Response {}

#[derive(Debug, Error)]
pub enum Error {
    #[error("error making request: {0:?}")]
    Request(reqwest::Error),

    #[error("error decoding response: {0:?}")]
    ResponseDecode(serde_json::Error),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Request(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::ResponseDecode(e)
    }
}

pub type Result<O> = std::result::Result<O, Error>;
