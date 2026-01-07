use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use miette::{Diagnostic, SourceSpan};
use oauth2::HttpClientError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// The type of rate limit that was exceeded.
///
/// Xero enforces multiple rate limits:
/// - **Minute limit**: 60 calls per minute per tenant
/// - **Daily limit**: 5000 calls per day per tenant
/// - **App minute limit**: 10,000 calls per minute across all tenants
///
/// This enum is populated from the `X-Rate-Limit-Problem` response header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitType {
    /// Per-tenant minute limit exceeded (60 calls/minute)
    Minute,
    /// Per-tenant daily limit exceeded (5000 calls/day)
    Daily,
    /// App-wide minute limit exceeded (10,000 calls/minute across all tenants)
    AppMinute,
    /// Unknown or unrecognized limit type
    Unknown(String),
}

impl RateLimitType {
    /// Parse the rate limit type from the `X-Rate-Limit-Problem` header value.
    #[must_use]
    pub fn from_header(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "minute" => Self::Minute,
            "daily" => Self::Daily,
            "appminute" => Self::AppMinute,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl fmt::Display for RateLimitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minute => write!(f, "minute (60 calls/min per tenant)"),
            Self::Daily => write!(f, "daily (5000 calls/day per tenant)"),
            Self::AppMinute => write!(f, "app minute (10000 calls/min across all tenants)"),
            Self::Unknown(s) => write!(f, "unknown ({s})"),
        }
    }
}

/// HTTP response context for debugging API errors.
///
/// This struct captures the essential information needed to debug failed API requests,
/// particularly deserialization errors where the response body is crucial for understanding
/// what Xero returned (e.g., HTML error pages, maintenance messages, etc.).
#[derive(Debug, Clone)]
pub struct ResponseContext {
    /// The URL that was called
    pub url: String,
    /// The HTTP method used (GET, POST, PUT, DELETE)
    pub method: String,
    /// The HTTP status code returned
    pub status_code: reqwest::StatusCode,
    /// The raw response body (truncated if too large)
    pub response_body: String,
    /// The expected entity type being deserialized
    pub entity_type: String,
}

impl ResponseContext {
    /// Maximum length for response body in error context (2KB)
    pub const MAX_BODY_LENGTH: usize = 2000;

    /// Create a new ResponseContext, truncating body if needed
    #[must_use]
    pub fn new(
        url: String,
        method: &str,
        status_code: reqwest::StatusCode,
        response_body: String,
        entity_type: String,
    ) -> Self {
        let truncated_body = if response_body.len() > Self::MAX_BODY_LENGTH {
            format!(
                "{}... [truncated, total {} bytes]",
                &response_body[..Self::MAX_BODY_LENGTH],
                response_body.len()
            )
        } else {
            response_body
        };

        Self {
            url,
            method: method.to_string(),
            status_code,
            response_body: truncated_body,
            entity_type,
        }
    }
}

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
/// - `Invoice`: Contains invoice ID and optional invoice number
/// - `Contact`: Contains contact ID
/// - `Item`: Contains item ID and optional code
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
    /// Invoice validation error with invoice identification
    Invoice {
        #[serde(rename = "InvoiceID")]
        invoice_id: Uuid,
        #[serde(rename = "InvoiceNumber")]
        invoice_number: Option<String>,
        #[serde(rename = "Type")]
        invoice_type: Option<String>,
        #[serde(rename = "Status")]
        status: Option<String>,
    },
    /// Contact validation error with contact identification
    Contact {
        #[serde(rename = "ContactID")]
        contact_id: Uuid,
        #[serde(rename = "Name")]
        name: Option<String>,
    },
    /// Item validation error with item identification
    Item {
        #[serde(rename = "ItemID")]
        item_id: Uuid,
        #[serde(rename = "Code")]
        code: Option<String>,
    },
    /// Purchase order validation error
    PurchaseOrder {
        #[serde(rename = "PurchaseOrderID")]
        purchase_order_id: Uuid,
    },
    /// Quote validation error
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

/// Format an OAuth2 error with detailed information, including raw response body for Parse errors.
fn format_oauth2_error(
    error: &oauth2::RequestTokenError<HttpClientError<reqwest::Error>, OAuth2ErrorResponse>,
) -> String {
    use oauth2::RequestTokenError;

    match error {
        RequestTokenError::ServerResponse(resp) => {
            format!("OAuth2 server error: {resp}")
        }
        RequestTokenError::Request(req_err) => {
            format!("OAuth2 request failed: {req_err:?}")
        }
        RequestTokenError::Parse(serde_err, raw_body) => {
            let body_str = String::from_utf8_lossy(raw_body);
            // Truncate very long responses
            let truncated = if body_str.len() > 500 {
                format!("{}... (truncated)", &body_str[..500])
            } else {
                body_str.to_string()
            };
            format!("OAuth2 token response parse error: {serde_err} - raw response: {truncated}")
        }
        RequestTokenError::Other(msg) => {
            format!("OAuth2 error: {msg}")
        }
    }
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

    /// Failed to parse the API response as JSON.
    ///
    /// This error includes the full HTTP response context for debugging.
    /// The response body is displayed with the error position highlighted.
    ///
    /// **Breaking Change (v0.2.0-alpha.13):** Changed from tuple variant
    /// `DeserializationError(serde_json::Error, Option<String>)` to struct variant.
    /// Use pattern matching with `{ source, context, .. }` instead.
    #[error(
        "Failed to parse {entity_type} response from {method} {url} (HTTP {status_code}): {source}"
    )]
    #[diagnostic(
        code(xero_rs::deserialization_error),
        help(
            "Xero returned non-JSON data. This often indicates an API outage, authentication issue, or rate limiting. Check the response body below."
        ),
        url("https://developer.xero.com/documentation/api/api-overview")
    )]
    DeserializationError {
        /// The underlying JSON parsing error
        #[source]
        source: serde_json::Error,
        /// The response body for miette source display
        #[source_code]
        response_body: String,
        /// Label pointing to error position in the response
        #[label("parse error here")]
        error_span: SourceSpan,
        /// Full HTTP response context
        context: ResponseContext,
        /// Entity type for display (from type name)
        entity_type: String,
        /// HTTP method for display
        method: String,
        /// URL for display
        url: String,
        /// Status code for display
        status_code: String,
    },

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
    #[error("{}", format_oauth2_error(.0))]
    #[diagnostic(
        code(xero_rs::oauth2_error),
        help("Verify your OAuth2 configuration and credentials")
    )]
    OAuth2(oauth2::RequestTokenError<HttpClientError<reqwest::Error>, OAuth2ErrorResponse>),

    /// Rate limit exceeded (HTTP 429 Too Many Requests)
    ///
    /// The `limit_type` field identifies which rate limit was exceeded:
    /// - `Minute`: 60 calls per minute per tenant
    /// - `Daily`: 5000 calls per day per tenant
    /// - `AppMinute`: 10,000 calls per minute across all tenants
    #[error("rate limit exceeded ({limit_type}): retry after {retry_after:?}")]
    #[diagnostic(
        code(xero_rs::rate_limit_exceeded),
        help(
            "The Xero API rate limit has been exceeded. Wait and retry, or implement request throttling."
        )
    )]
    RateLimitExceeded {
        /// The type of rate limit that was exceeded
        limit_type: RateLimitType,
        /// How long to wait before retrying (from Retry-After header)
        retry_after: Option<Duration>,
        status_code: reqwest::StatusCode,
        url: String,
        response_body: Option<String>,
    },
}

impl Error {
    /// Create a DeserializationError with full HTTP context.
    ///
    /// This constructor captures all the debugging information needed to diagnose
    /// why Xero returned non-JSON data (e.g., HTML error pages, maintenance messages).
    ///
    /// # Arguments
    ///
    /// * `source` - The underlying serde_json parsing error
    /// * `url` - The URL that was called
    /// * `method` - The HTTP method (GET, POST, PUT, DELETE)
    /// * `status_code` - The HTTP status code returned
    /// * `response_body` - The raw response body
    /// * `entity_type` - The type name being deserialized
    #[must_use]
    pub fn deserialization_error(
        source: serde_json::Error,
        url: String,
        method: &str,
        status_code: reqwest::StatusCode,
        response_body: String,
        entity_type: String,
    ) -> Self {
        // Calculate span: start at error column, extend ~100 chars or to end of body
        let col = source.column();
        let start = col.saturating_sub(1);
        let len = response_body.len().saturating_sub(start).min(100).max(1);
        let error_span = SourceSpan::new(start.into(), len.into());

        let context = ResponseContext::new(
            url.clone(),
            method,
            status_code,
            response_body.clone(),
            entity_type.clone(),
        );

        Self::DeserializationError {
            source,
            response_body,
            error_span,
            context,
            entity_type,
            method: method.to_string(),
            url,
            status_code: status_code.to_string(),
        }
    }

    /// Get the response context if this error has HTTP context.
    ///
    /// Returns `Some(&ResponseContext)` for:
    /// - `DeserializationError`
    #[must_use]
    pub fn response_context(&self) -> Option<&ResponseContext> {
        match self {
            Self::DeserializationError { context, .. } => Some(context),
            _ => None,
        }
    }

    /// Get the response body for errors that capture it.
    #[must_use]
    pub fn response_body(&self) -> Option<&str> {
        match self {
            Self::DeserializationError { response_body, .. } => Some(response_body),
            Self::NotFound { response_body, .. } => response_body.as_deref(),
            Self::RateLimitExceeded { response_body, .. } => response_body.as_deref(),
            _ => None,
        }
    }

    /// Get the URL for errors that capture it.
    #[must_use]
    pub fn url(&self) -> Option<&str> {
        match self {
            Self::DeserializationError { url, .. } => Some(url),
            Self::NotFound { url, .. } => Some(url),
            Self::RateLimitExceeded { url, .. } => Some(url),
            _ => None,
        }
    }

    /// Get the HTTP status code for errors that capture it.
    #[must_use]
    pub fn status_code(&self) -> Option<reqwest::StatusCode> {
        match self {
            Self::DeserializationError { context, .. } => Some(context.status_code),
            Self::NotFound { status_code, .. } => Some(*status_code),
            Self::RateLimitExceeded { status_code, .. } => Some(*status_code),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Request(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        // For standalone serde errors without HTTP context, create minimal error
        let col = e.column().saturating_sub(1);
        Self::DeserializationError {
            error_span: SourceSpan::new(col.into(), 1usize.into()),
            entity_type: "unknown".to_string(),
            method: "unknown".to_string(),
            url: "unknown".to_string(),
            status_code: "unknown".to_string(),
            context: ResponseContext::new(
                "unknown".to_string(),
                "unknown",
                reqwest::StatusCode::OK,
                String::new(),
                "unknown".to_string(),
            ),
            response_body: String::new(),
            source: e,
        }
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
