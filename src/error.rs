use thiserror::Error;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {

}

impl oauth2::ErrorResponse for ErrorResponse {
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("error making request: {0:?}")]
    Request(reqwest::Error),

    #[error("error decoding response: {0:?}")]
    ResponseDecode(serde_json::Error)
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

pub type XeroResult<O> = Result<O, Error>;
