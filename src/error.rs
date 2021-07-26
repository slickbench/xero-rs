use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ErrorResponse {}

impl oauth2::ErrorResponse for OAuth2ErrorResponse {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(clippy::module_name_repetitions)]
pub enum ErrorType {
    ValidationException,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(clippy::module_name_repetitions)]
pub struct ValidationError {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "Type", rename_all = "UPPERCASE")]
pub enum ValidationExceptionElementObject {
    #[serde(rename_all = "PascalCase")]
    PurchaseOrder {
        #[serde(rename = "PurchaseOrderID")]
        purchase_order_id: Uuid,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ValidationExceptionElement {
    pub validation_errors: Vec<ValidationError>,
    #[serde(flatten)]
    pub object: ValidationExceptionElementObject,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Response {
    error_number: u64,
    r#type: ErrorType,
    message: String,
    elements: Vec<ValidationExceptionElement>,
}

/// Errors that can occur when interacting with the Xero API.
#[derive(Debug, Error)]
pub enum Error {
    #[error("error making request: {0:?}")]
    Request(reqwest::Error),

    #[error("error decoding response: {0:?} | body: {1:?}")]
    DeserializationError(serde_json::Error, Option<String>),

    #[error("unexpected response status: {0:?} | body: {1:?}")]
    UnexpectedResponseStatus(StatusCode, Option<String>),

    #[error("object not found")]
    NotFound,

    #[error("endpoint could not be parsed as a URL")]
    InvalidEndpoint,

    #[error("encountered validation exception: {0:#?}")]
    XeroError(Response),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Request(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::DeserializationError(e, None)
    }
}

pub type Result<O> = std::result::Result<O, Error>;
