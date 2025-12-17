use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use miette::Diagnostic;
use oauth2::HttpClientError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// OAuth2 error response from Xero's identity server.
///
/// This captures both standard OAuth2 error responses (RFC 6749 Section 5.2)
/// and Xero-specific error fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ErrorResponse {
    /// Standard OAuth2 error code (e.g., "invalid_client", "invalid_grant")
    #[serde(default)]
    pub error: Option<String>,
    /// Human-readable error description
    #[serde(default)]
    pub error_description: Option<String>,
    /// URI with more information about the error
    #[serde(default)]
    pub error_uri: Option<String>,
    /// Xero-specific OAuth problem field (legacy OAuth1-style errors)
    #[serde(default)]
    pub oauth_problem: Option<String>,
    /// Additional advice from Xero
    #[serde(default)]
    pub oauth_problem_advice: Option<String>,
}

impl oauth2::ErrorResponse for OAuth2ErrorResponse {}

impl fmt::Display for OAuth2ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(error) = &self.error {
            write!(f, "OAuth2 error: {error}")?;
            if let Some(desc) = &self.error_description {
                write!(f, " - {desc}")?;
            }
        } else if let Some(problem) = &self.oauth_problem {
            write!(f, "OAuth problem: {problem}")?;
            if let Some(advice) = &self.oauth_problem_advice {
                write!(f, " - {advice}")?;
            }
        } else {
            write!(f, "OAuth2 error occurred (no details available)")?;
        }
        Ok(())
    }
}

/// Xero API error types.
///
/// This enum represents the different types of errors returned by the Xero API.
/// The `Type` field in the JSON response is used as a discriminator.
///
/// # ValidationException
/// The most common error type, returned when validation fails on submitted entities.
///
/// **Breaking Change (v0.2.0-alpha.4):** The `elements` field no longer uses `#[serde(default)]`.
/// If the API returns a ValidationException without an Elements array, deserialization will now fail
/// instead of silently defaulting to an empty vector. This improves error visibility.
///
/// ## Example Response
/// ```json
/// {
///   "ErrorNumber": 10,
///   "Type": "ValidationException",
///   "Message": "A validation exception occurred",
///   "Elements": [{
///     "QuoteID": "efcef70f-f4f9-4baf-83b6-b5eac086c91b",
///     "Status": "ACCEPTED",
///     "ValidationErrors": [
///       {"Message": "Contact requires a valid ContactId or ContactName"}
///     ]
///   }]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "Type", rename_all = "PascalCase")]
#[allow(clippy::module_name_repetitions)]
pub enum ErrorType {
    /// Validation error with details about which entity fields failed validation.
    ///
    /// # Fields
    /// - `elements`: Array of validation errors per entity. **No longer defaults to empty array** - missing Elements will cause deserialization error.
    /// - `timesheets`: Optional timesheet-specific validation errors (defaults to None).
    ValidationException {
        #[serde(rename = "Elements")]
        elements: Vec<ValidationExceptionElement>,
        #[serde(rename = "Timesheets", default)]
        timesheets: Option<Vec<TimesheetValidationError>>,
    },
    PostDataInvalidException,
    QueryParseException,
    ObjectNotFoundException,
    OrganisationOfflineException,
    UnauthorisedException,
    NoDataProcessedException,
    UnsupportedMediaTypeException,
    MethodNotAllowedException,
    InternalServerException,
    NotImplementedException,
    NotAvailableException,
    RateLimitExceededException,
    SystemUnavailableException,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(clippy::module_name_repetitions)]
pub struct ValidationError {
    pub message: String,
}

/// The object being validated in a ValidationException.
///
/// Xero returns validation errors with the entity that failed validation.
/// This enum uses `#[serde(untagged)]` to match based on field presence,
/// since the API doesn't include a "Type" discriminator field.
///
/// # Entity Variants
/// - `PurchaseOrder`: Contains purchase order ID
/// - `Quote`: Contains quote ID and optional status
/// - `Unknown`: Fallback for unsupported entity types, preserves raw data
///
/// # Example Response
/// ```json
/// {
///   "QuoteID": "efcef70f-f4f9-4baf-83b6-b5eac086c91b",
///   "Status": "ACCEPTED",
///   "ValidationErrors": [{"Message": "Contact requires a valid ContactId"}]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ValidationExceptionElementObject {
    PurchaseOrder {
        #[serde(rename = "PurchaseOrderID")]
        purchase_order_id: Uuid,
    },
    Quote {
        #[serde(rename = "QuoteID")]
        quote_id: Uuid,
        #[serde(rename = "Status")]
        status: Option<String>,
    },
    /// Fallback variant for entity types not yet explicitly supported.
    /// Preserves the raw JSON for debugging and future compatibility.
    Unknown(serde_json::Value),
}

/// A validation error element containing the entity being validated and its errors.
///
/// Each element combines:
/// - The entity that failed validation (flattened from `object`)
/// - The validation error messages for that entity
///
/// The `object` field is flattened, meaning its fields appear at the same level
/// as `validation_errors` in the JSON response.
///
/// # Example
/// ```json
/// {
///   "QuoteID": "efcef70f-f4f9-4baf-83b6-b5eac086c91b",
///   "Status": "ACCEPTED",
///   "ValidationErrors": [{"Message": "Contact requires a valid ContactId"}]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ValidationExceptionElement {
    /// The validation error messages for this entity.
    pub validation_errors: Vec<ValidationError>,
    /// The entity being validated (Quote, PurchaseOrder, or Unknown).
    /// Fields from this enum variant are flattened into the parent struct.
    #[serde(flatten)]
    pub object: ValidationExceptionElementObject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
pub struct Response {
    #[serde(default)]
    pub error_number: Option<u64>,
    #[serde(default)]
    pub status: Option<u64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    #[serde(default)]
    pub instance: Option<Uuid>,
    #[serde(flatten)]
    pub error: ErrorType,
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Xero API Error ({}): {}",
            self.error_number.unwrap_or(0),
            self.message
                .as_deref()
                .or(self.title.as_deref())
                .or(self.detail.as_deref())
                .unwrap_or("Unknown")
        )?;

        // Add additional details based on error type
        match &self.error {
            ErrorType::ValidationException {
                elements,
                timesheets,
            } => {
                if !elements.is_empty() {
                    write!(f, "\nValidation errors:")?;
                    for element in elements {
                        for error in &element.validation_errors {
                            write!(f, "\n  - {}", error.message)?;
                        }
                    }
                }
                if let Some(timesheet_errors) = timesheets
                    && !timesheet_errors.is_empty()
                {
                    write!(f, "\nTimesheet errors:")?;
                    for ts_error in timesheet_errors {
                        for error in &ts_error.validation_errors {
                            write!(f, "\n  - {}", error.message)?;
                        }
                    }
                }
            }
            ErrorType::QueryParseException => {
                write!(
                    f,
                    "\nThe query string could not be parsed. Check for missing quotes or invalid syntax."
                )?;
            }
            _ => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
pub struct ForbiddenResponse {
    r#type: Option<String>,
    title: String,
    status: u16,
    detail: String,
    instance: Uuid,
    extensions: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Error, Diagnostic)]
pub enum Error {
    #[error("error making request: {0:?}")]
    #[diagnostic(
        code(xero_rs::request_error),
        help("Check your network connection and Xero API availability")
    )]
    Request(#[source] reqwest::Error),

    #[error("invalid filename")]
    #[diagnostic(
        code(xero_rs::invalid_filename),
        help("Ensure the filename is valid and compatible with Xero API requirements")
    )]
    InvalidFilename,

    #[error("attachment too large")]
    #[diagnostic(
        code(xero_rs::attachment_too_large),
        help("Reduce the attachment size to comply with Xero API limits")
    )]
    AttachmentTooLarge,

    #[error("error decoding response: {0:?}")]
    #[diagnostic(
        code(xero_rs::deserialization_error),
        help("The API returned data in an unexpected format")
    )]
    DeserializationError(#[source] serde_json::Error, Option<String>),

    #[error("object not found: {entity} (url: {url})")]
    #[diagnostic(
        code(xero_rs::not_found),
        help("Verify that the {entity} exists and that you have permission to access it")
    )]
    NotFound {
        entity: String,
        url: String,
        status_code: reqwest::StatusCode,
        response_body: Option<String>,
    },

    #[error("endpoint could not be parsed as a URL")]
    #[diagnostic(
        code(xero_rs::invalid_endpoint),
        help("Check that the API endpoint URL is correctly formatted")
    )]
    InvalidEndpoint,

    /// A standard error returned while interacting with the API such as a `ValidationException`.
    #[error("{0}")]
    #[diagnostic(
        code(xero_rs::api_validation),
        help("Review the validation errors returned by the Xero API")
    )]
    API(Response),

    /// An error returned when the user is forbidden by something like an unsuccessful
    /// authentication.
    #[error("encountered forbidden response: {0:#?}")]
    #[diagnostic(
        code(xero_rs::forbidden),
        help("Check your authentication credentials and permissions for the requested resource")
    )]
    Forbidden(Box<ForbiddenResponse>),

    /// An error returned during `OAuth2` operations
    #[error("oauth2 error: {0:?}")]
    #[diagnostic(
        code(xero_rs::oauth2_error),
        help("Verify your OAuth2 configuration and credentials")
    )]
    OAuth2(oauth2::RequestTokenError<HttpClientError<reqwest::Error>, OAuth2ErrorResponse>),

    /// Rate limit exceeded (HTTP 429 Too Many Requests)
    #[error("rate limit exceeded: retry after {retry_after:?}")]
    #[diagnostic(
        code(xero_rs::rate_limit_exceeded),
        help(
            "The Xero API rate limit has been exceeded. Wait and retry, or implement request throttling."
        )
    )]
    RateLimitExceeded {
        retry_after: Option<Duration>,
        status_code: reqwest::StatusCode,
        url: String,
        response_body: Option<String>,
    },
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

impl From<ForbiddenResponse> for Error {
    fn from(response: ForbiddenResponse) -> Self {
        Self::Forbidden(Box::new(response))
    }
}

/// Type alias for results from this crate.
///
/// This is already a Miette diagnostic result due to the implementation of
/// the Diagnostic trait for the Error type.
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
