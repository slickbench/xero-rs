use std::collections::HashMap;
use std::fmt;

use oauth2::HttpClientError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ErrorResponse {}

impl oauth2::ErrorResponse for OAuth2ErrorResponse {}

impl fmt::Display for OAuth2ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OAuth2 error occurred")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "Type", rename_all = "PascalCase")]
#[allow(clippy::module_name_repetitions)]
pub enum ErrorType {
    ValidationException {
        #[serde(default)]
        elements: Vec<ValidationExceptionElement>,
        #[serde(default)]
        timesheets: Option<Vec<TimesheetValidationError>>,
    },
    PostDataInvalidException,
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
    message: String,
    #[serde(flatten)]
    error: ErrorType,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ForbiddenResponse {
    r#type: Option<String>,
    title: String,
    status: u16,
    detail: String,
    instance: Uuid,
    extensions: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TimesheetValidationError {
    #[serde(default)]
    pub validation_errors: Vec<ValidationError>,
    #[serde(rename = "EmployeeID")]
    pub employee_id: Option<Uuid>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: Option<String>,
    pub hours: Option<f64>,
    #[serde(default)]
    pub timesheet_lines: Vec<serde_json::Value>,
}

/// Errors that can occur when interacting with the Xero API.
#[derive(Debug, Error)]
pub enum Error {
    #[error("error making request: {0:?}")]
    Request(reqwest::Error),

    #[error("invalid filename")]
    InvalidFilename,

    #[error("attachment too large")]
    AttachmentTooLarge,

    #[error("error decoding response: {0:?} | body: {1:#?}")]
    DeserializationError(serde_json::Error, Option<String>),

    #[error("object not found: {entity} (url: {url})")]
    NotFound {
        entity: String,
        url: String,
        status_code: reqwest::StatusCode,
        response_body: Option<String>,
    },

    #[error("endpoint could not be parsed as a URL")]
    InvalidEndpoint,

    /// A standard error returned while interacting with the API such as a `ValidationException`.
    #[error("encountered validation exception: {0:#?}")]
    API(Response),

    /// An error returned when the user is forbidden by something like an unsuccessful
    /// authentication.
    #[error("encountered forbidden response: {0:#?}")]
    Forbidden(ForbiddenResponse),

    /// An error returned during `OAuth2` operations
    #[error("oauth2 error: {0:?}")]
    OAuth2(oauth2::RequestTokenError<HttpClientError<reqwest::Error>, OAuth2ErrorResponse>),
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

impl From<oauth2::RequestTokenError<HttpClientError<reqwest::Error>, OAuth2ErrorResponse>>
    for Error
{
    fn from(
        e: oauth2::RequestTokenError<HttpClientError<reqwest::Error>, OAuth2ErrorResponse>,
    ) -> Self {
        Self::OAuth2(e)
    }
}

pub type Result<O> = std::result::Result<O, Error>;

/// Macro to handle common error mapping patterns
#[macro_export]
macro_rules! handle_api_response {
    ($response:expr, $entity_type:expr) => {
        match $response {
            Ok(response) => Ok(response),
            Err(e) => {
                tracing::error!("API error for {}: {:?}", $entity_type, e);
                Err(e)
            }
        }
    };
}
