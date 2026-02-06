use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::Path;
use time::Date;
use tracing_error::SpanTrace;
use uuid::Uuid;

use crate::{
    Client,
    contact::{Contact, ContactIdentifier},
    endpoints::XeroEndpoint,
    entities::{EntityEndpoint, MutationResponse, endpoint_utils},
    error::{Error, Result},
    line_item::{self, LineAmountType, LineItem},
    utils::date_format::{xero_date_format, xero_date_format_option},
};

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/Quotes/";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Draft,
    Deleted,
    Sent,
    Declined,
    Accepted,
    Invoiced,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Quote {
    pub contact: Contact,
    pub date: String,
    pub expiry_date: Option<String>,
    pub status: Status,
    pub line_amount_types: LineAmountType,
    pub line_items: Vec<LineItem>,
    pub sub_total: Decimal,
    pub total_tax: Decimal,
    pub total: Decimal,
    pub total_discount: Option<Decimal>,
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: String,
    pub currency_code: String,
    pub currency_rate: Option<Decimal>,
    #[serde(rename = "QuoteID")]
    pub quote_id: Uuid,
    pub quote_number: String,
    pub reference: Option<String>,
    pub branding_theme_id: Option<Uuid>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub terms: Option<String>,
    #[serde(default)]
    pub has_attachments: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ListResponse {
    pub quotes: Vec<Quote>,
}

impl From<ListResponse> for Vec<Quote> {
    fn from(response: ListResponse) -> Self {
        response.quotes
    }
}

/// Parameters for filtering quote list results
#[derive(Debug, Serialize, Default)]
pub struct ListParameters {
    /// Filter for quotes after a particular date
    #[serde(
        rename = "DateFrom",
        skip_serializing_if = "Option::is_none",
        with = "xero_date_format_option"
    )]
    pub date_from: Option<Date>,

    /// Filter for quotes before a particular date
    #[serde(
        rename = "DateTo",
        skip_serializing_if = "Option::is_none",
        with = "xero_date_format_option"
    )]
    pub date_to: Option<Date>,

    /// Filter for quotes expiring after a particular date
    #[serde(
        rename = "ExpiryDateFrom",
        skip_serializing_if = "Option::is_none",
        with = "xero_date_format_option"
    )]
    pub expiry_date_from: Option<Date>,

    /// Filter for quotes expiring before a particular date
    #[serde(
        rename = "ExpiryDateTo",
        skip_serializing_if = "Option::is_none",
        with = "xero_date_format_option"
    )]
    pub expiry_date_to: Option<Date>,

    /// Filter for quotes belonging to a particular contact
    #[serde(rename = "ContactID", skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<Uuid>,

    /// Filter for quotes of a particular Status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,

    /// Pagination parameter (1-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,

    /// Order by any element
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,

    /// Filter by quote number
    #[serde(rename = "QuoteNumber", skip_serializing_if = "Option::is_none")]
    pub quote_number: Option<String>,

    /// Unit price decimal places (4 or 2, defaults to 2 if not specified)
    ///
    /// By default, the API accepts unit prices (UnitAmount) to two decimal places.
    /// Set to 4 to get unit prices with 4 decimal precision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unitdp: Option<u8>,
}

impl ListParameters {
    /// Create a new builder for `ListParameters`
    #[must_use]
    pub fn builder() -> Self {
        Self::default()
    }

    /// Set the `date_from` filter
    #[must_use]
    pub fn with_date_from(mut self, date: Date) -> Self {
        self.date_from = Some(date);
        self
    }

    /// Set the `date_to` filter
    #[must_use]
    pub fn with_date_to(mut self, date: Date) -> Self {
        self.date_to = Some(date);
        self
    }

    /// Set the `expiry_date_from` filter
    #[must_use]
    pub fn with_expiry_date_from(mut self, date: Date) -> Self {
        self.expiry_date_from = Some(date);
        self
    }

    /// Set the `expiry_date_to` filter
    #[must_use]
    pub fn with_expiry_date_to(mut self, date: Date) -> Self {
        self.expiry_date_to = Some(date);
        self
    }

    /// Set the `contact_id` filter
    #[must_use]
    pub fn with_contact_id(mut self, id: Uuid) -> Self {
        self.contact_id = Some(id);
        self
    }

    /// Set the status filter
    #[must_use]
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the page number
    #[must_use]
    pub fn with_page(mut self, page: i32) -> Self {
        self.page = Some(page);
        self
    }

    /// Set the order clause
    #[must_use]
    pub fn with_order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    /// Set the `quote_number` filter
    #[must_use]
    pub fn with_quote_number(mut self, number: impl Into<String>) -> Self {
        self.quote_number = Some(number.into());
        self
    }

    /// Set unit price decimal places (4 for 4 decimal precision, 2 for default)
    ///
    /// By default, the API returns unit prices with 2 decimal places.
    /// Use this to request 4 decimal places for more precision.
    #[must_use]
    pub fn with_unitdp(mut self, unitdp: u8) -> Self {
        self.unitdp = Some(unitdp);
        self
    }
}

/// Information required to create or update a quote
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct QuoteBuilder {
    /// The contact the quote is for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<ContactIdentifier>,

    /// The quote date
    #[serde(with = "xero_date_format")]
    pub date: Date,

    /// The quote's expiry date
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub expiry_date: Option<Date>,

    /// Line items for the quote
    pub line_items: Vec<line_item::Builder>,

    /// Tax calculation type
    pub line_amount_types: LineAmountType,

    /// Quote title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Quote summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Quote terms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms: Option<String>,

    /// Quote reference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,

    /// The quote's currency code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_code: Option<String>,

    /// The quote's branding theme ID
    #[serde(rename = "BrandingThemeID", skip_serializing_if = "Option::is_none")]
    pub branding_theme_id: Option<Uuid>,

    /// The quote's ID (used for updates)
    #[serde(rename = "QuoteID", skip_serializing_if = "Option::is_none")]
    pub quote_id: Option<Uuid>,

    /// The quote's number (e.g., "QU-0001")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_number: Option<String>,

    /// The quote's status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
}

/// Create a request wrapper for quotes
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct QuoteWrapper<'a> {
    pub quotes: Vec<&'a QuoteBuilder>,
}

/// History record for a quote
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecord {
    /// The details of the history record
    pub details: String,

    /// The date and time of the history record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_utc: Option<String>,

    /// The user who created the history record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// The changes made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<String>,
}

/// Wrapper for history records response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecords {
    pub history_records: Vec<HistoryRecord>,
}

/// Wrapper for posting history records
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecordsRequest {
    pub history_records: Vec<HistoryRecord>,
}

/// Attachment details for a quote
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Attachment {
    #[serde(rename = "AttachmentID")]
    pub attachment_id: Uuid,
    pub file_name: String,
    pub url: String,
    pub mime_type: String,
    pub content_length: i64,
}

/// Attachments response wrapper
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Attachments {
    pub attachments: Vec<Attachment>,
}

/// Implementation of `EntityEndpoint` for Quote
impl EntityEndpoint<Quote, ListParameters> for Quote {
    fn endpoint() -> &'static str {
        ENDPOINT
    }

    async fn get(client: &Client, id: Uuid) -> Result<Quote> {
        endpoint_utils::get::<Quote, ListResponse>(client, ENDPOINT, id, "Quote").await
    }

    async fn list(client: &Client, params: ListParameters) -> Result<Vec<Quote>> {
        endpoint_utils::list::<Quote, ListResponse, _>(client, ENDPOINT, &params).await
    }
}

/// Retrieve a list of quotes with filtering.
#[instrument(skip(client))]
pub async fn list(client: &Client, params: ListParameters) -> Result<Vec<Quote>> {
    Quote::list(client, params).await
}

/// Retrieve a list of all quotes without filtering.
#[instrument(skip(client))]
pub async fn list_all(client: &Client) -> Result<Vec<Quote>> {
    Quote::list(client, ListParameters::default()).await
}

/// Retrieve a single quote by ID
#[instrument(skip(client))]
pub async fn get(client: &Client, quote_id: Uuid) -> Result<Quote> {
    let endpoint = XeroEndpoint::Custom(vec!["Quotes".to_string(), quote_id.to_string()]);

    let endpoint_clone = endpoint.clone();
    let empty_tuple = ();
    let response: ListResponse = client.get_endpoint(endpoint, &empty_tuple).await?;

    response.quotes.into_iter().next().ok_or(Error::NotFound {
        entity: "Quote".to_string(),
        url: endpoint_clone.to_string(),
        status_code: reqwest::StatusCode::NOT_FOUND,
        response_body: Some(format!("Quote with ID {quote_id} not found")),
        span_trace: SpanTrace::capture(),
    })
}

/// Create one or more quotes.
#[instrument(skip(client, quote))]
pub async fn create(
    client: &Client,
    quote: &QuoteBuilder,
    options: &crate::MutationOptions,
) -> Result<Quote> {
    let request = QuoteWrapper {
        quotes: vec![quote],
    };

    let response: MutationResponse = client
        .put_endpoint_with_options(XeroEndpoint::Quotes, &request, options)
        .await?;

    // Extract quote from response
    response
        .data
        .get_quotes()
        .and_then(|quotes| quotes.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Quote".to_string(),
            url: XeroEndpoint::Quotes.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some("No quote returned in response".to_string()),
            span_trace: SpanTrace::capture(),
        })
}

/// Update or create one or more quotes.
#[instrument(skip(client, quote))]
pub async fn update_or_create(
    client: &Client,
    quote: &QuoteBuilder,
    options: &crate::MutationOptions,
) -> Result<Quote> {
    let request = QuoteWrapper {
        quotes: vec![quote],
    };

    let response: MutationResponse = client
        .post_endpoint_with_options(XeroEndpoint::Quotes, &request, options)
        .await?;

    // Extract quote from response
    response
        .data
        .get_quotes()
        .and_then(|quotes| quotes.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Quote".to_string(),
            url: XeroEndpoint::Quotes.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some("No quote returned in response".to_string()),
            span_trace: SpanTrace::capture(),
        })
}

/// Update a specific quote.
#[instrument(skip(client, quote))]
pub async fn update(
    client: &Client,
    quote_id: Uuid,
    quote: &QuoteBuilder,
    options: &crate::MutationOptions,
) -> Result<Quote> {
    let mut updatable_quote = quote.clone();
    updatable_quote.quote_id = Some(quote_id);

    let request = QuoteWrapper {
        quotes: vec![&updatable_quote],
    };

    let endpoint = XeroEndpoint::Quote(quote_id);
    let response: MutationResponse = client
        .post_endpoint_with_options(endpoint.clone(), &request, options)
        .await?;

    // Extract quote from response
    response
        .data
        .get_quotes()
        .and_then(|quotes| quotes.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Quote".to_string(),
            url: endpoint.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some(format!("Quote with ID {quote_id} not found")),
            span_trace: SpanTrace::capture(),
        })
}

/// Retrieve history records for a quote
#[instrument(skip(client))]
pub async fn get_history(client: &Client, quote_id: Uuid) -> Result<Vec<HistoryRecord>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Quotes".to_string(),
        quote_id.to_string(),
        "History".to_string(),
    ]);

    let empty_tuple = ();
    let response: HistoryRecords = client.get_endpoint(endpoint, &empty_tuple).await?;

    Ok(response.history_records)
}

/// Create a history record for a specific quote.
#[instrument(skip(client))]
pub async fn create_history(
    client: &Client,
    quote_id: Uuid,
    details: &str,
) -> Result<Vec<HistoryRecord>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Quotes".to_string(),
        quote_id.to_string(),
        "History".to_string(),
    ]);

    let history_record = HistoryRecord {
        details: details.to_string(),
        date_utc: None,
        user: None,
        changes: None,
    };

    let request = HistoryRecordsRequest {
        history_records: vec![history_record],
    };

    let response: HistoryRecords = client.put_endpoint(endpoint, &request).await?;

    Ok(response.history_records)
}

/// Retrieve a quote as a PDF file.
#[instrument(skip(client))]
pub async fn get_pdf(client: &Client, quote_id: Uuid) -> Result<Vec<u8>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Quotes".to_string(),
        quote_id.to_string(),
        "pdf".to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::GET, url)
        .await
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(Error::NotFound {
            entity: "Quote PDF".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to retrieve PDF for quote with ID {quote_id}"
            )),
            span_trace: SpanTrace::capture(),
        })
    }
}

/// List all attachments for a quote.
#[instrument(skip(client))]
pub async fn list_attachments(client: &Client, quote_id: Uuid) -> Result<Vec<Attachment>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Quotes".to_string(),
        quote_id.to_string(),
        "Attachments".to_string(),
    ]);

    let empty_tuple = ();
    let response: Attachments = client.get_endpoint(endpoint, &empty_tuple).await?;

    Ok(response.attachments)
}

/// Get a specific attachment by ID.
#[instrument(skip(client))]
pub async fn get_attachment(
    client: &Client,
    quote_id: Uuid,
    attachment_id: Uuid,
) -> Result<Vec<u8>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Quotes".to_string(),
        quote_id.to_string(),
        "Attachments".to_string(),
        attachment_id.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::GET, url)
        .await
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(Error::NotFound {
            entity: "Quote Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to retrieve attachment for quote with ID {quote_id}"
            )),
            span_trace: SpanTrace::capture(),
        })
    }
}

/// Get an attachment by filename.
#[instrument(skip(client))]
pub async fn get_attachment_by_filename(
    client: &Client,
    quote_id: Uuid,
    filename: &str,
) -> Result<Vec<u8>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Quotes".to_string(),
        quote_id.to_string(),
        "Attachments".to_string(),
        filename.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::GET, url)
        .await
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(Error::NotFound {
            entity: "Quote Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to retrieve attachment {filename} for quote with ID {quote_id}"
            )),
            span_trace: SpanTrace::capture(),
        })
    }
}

/// Upload an attachment to a quote.
#[instrument(skip(client, attachment_content))]
pub async fn upload_attachment(
    client: &Client,
    quote_id: Uuid,
    filename: &str,
    attachment_content: &[u8],
) -> Result<Attachment> {
    // Define constants first
    const MAX_ATTACHMENT_SIZE: usize = 25 * 1024 * 1024; // 25 MB

    // 1. Check if filename is valid
    if filename.is_empty() {
        return Err(Error::InvalidFilename);
    }

    // 2. Determine content type from filename extension
    let ext = Path::new(filename).extension().and_then(OsStr::to_str);

    let content_type = match ext {
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        // Add more mappings as needed
        _ => "application/octet-stream", // Default fallback
    };

    // 3. Validate attachment size (up to 25 MB)
    if attachment_content.len() > MAX_ATTACHMENT_SIZE {
        return Err(Error::AttachmentTooLarge);
    }

    // Create the endpoint URL using XeroEndpoint
    let endpoint = XeroEndpoint::Custom(vec![
        "Quotes".to_string(),
        quote_id.to_string(),
        "Attachments".to_string(),
        filename.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::PUT, url)
        .await
        .header(reqwest::header::CONTENT_TYPE, content_type)
        .header(reqwest::header::CONTENT_LENGTH, attachment_content.len())
        .body(attachment_content.to_vec())
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        let attachments: Attachments = response.json().await?;
        attachments
            .attachments
            .into_iter()
            .next()
            .ok_or(Error::NotFound {
                entity: "Quote Attachment".to_string(),
                url: endpoint.to_string(),
                status_code: status,
                response_body: Some("No attachment was returned after upload".to_string()),
                span_trace: SpanTrace::capture(),
            })
    } else {
        Err(Error::NotFound {
            entity: "Quote Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to upload attachment for quote with ID {quote_id}"
            )),
            span_trace: SpanTrace::capture(),
        })
    }
}

/// Update an existing attachment.
#[instrument(skip(client, attachment_content))]
pub async fn update_attachment(
    client: &Client,
    quote_id: Uuid,
    filename: &str,
    attachment_content: &[u8],
) -> Result<Attachment> {
    // Define constants first
    const MAX_ATTACHMENT_SIZE: usize = 25 * 1024 * 1024; // 25 MB

    // 1. Check if filename is valid
    if filename.is_empty() {
        return Err(Error::InvalidFilename);
    }

    // 2. Determine content type from filename extension
    let ext = Path::new(filename).extension().and_then(OsStr::to_str);

    let content_type = match ext {
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        // Add more mappings as needed
        _ => "application/octet-stream", // Default fallback
    };

    // 3. Validate attachment size (up to 25 MB)
    if attachment_content.len() > MAX_ATTACHMENT_SIZE {
        return Err(Error::AttachmentTooLarge);
    }

    // Create the endpoint URL using XeroEndpoint
    let endpoint = XeroEndpoint::Custom(vec![
        "Quotes".to_string(),
        quote_id.to_string(),
        "Attachments".to_string(),
        filename.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::POST, url)
        .await
        .header(reqwest::header::CONTENT_TYPE, content_type)
        .header(reqwest::header::CONTENT_LENGTH, attachment_content.len())
        .body(attachment_content.to_vec())
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        let attachments: Attachments = response.json().await?;
        attachments
            .attachments
            .into_iter()
            .next()
            .ok_or(Error::NotFound {
                entity: "Quote Attachment".to_string(),
                url: endpoint.to_string(),
                status_code: status,
                response_body: Some("No attachment was returned after update".to_string()),
                span_trace: SpanTrace::capture(),
            })
    } else {
        Err(Error::NotFound {
            entity: "Quote Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to update attachment for quote with ID {quote_id}"
            )),
            span_trace: SpanTrace::capture(),
        })
    }
}
