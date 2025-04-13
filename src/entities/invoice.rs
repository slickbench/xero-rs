use std::{ffi::OsStr, path::Path};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{Date, OffsetDateTime};
use url::Url;
use uuid::Uuid;

use crate::{
    contact::{Contact, ContactIdentifier},
    endpoints::XeroEndpoint,
    entities::{endpoint_utils, EntityEndpoint, MutationResponse},
    error::{Error, Result},
    line_item::{LineAmountType, LineItem},
    utils::date_format::{xero_date_format, xero_date_format_option, xero_datetime_format},
    Client,
};

use super::line_item;

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/Invoices/";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Type {
    #[serde(rename = "ACCPAY")]
    AccountsPayable,

    #[serde(rename = "ACCREC")]
    AccountsReceivable,
}

impl Default for Type {
    fn default() -> Self {
        Self::AccountsReceivable
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Draft,
    Submitted,
    Deleted,
    Authorised,
    Paid,
    Voided,
}

impl Default for Status {
    fn default() -> Self {
        Self::Draft
    }
}

/// Represents a payment applied to an invoice
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Payment {
    #[serde(rename = "PaymentID")]
    pub payment_id: Uuid,
    #[serde(with = "xero_date_format")]
    pub date: Date,
    pub amount: Decimal,
    pub reference: Option<String>,
    pub has_account: Option<bool>,
    pub has_validation_errors: Option<bool>,
}

/// Represents a credit note applied to an invoice
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreditNote {
    #[serde(rename = "ID")]
    pub id: Uuid,
    pub credit_note_number: String,
    pub has_errors: bool,
    pub applied_amount: Decimal,
    #[serde(with = "xero_datetime_format")]
    pub date: OffsetDateTime,
}

/// Represents a prepayment applied to an invoice
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Prepayment {
    #[serde(rename = "PrepaymentID")]
    pub prepayment_id: Uuid,
    pub prepayment_number: Option<String>,
    pub reference: Option<String>,
    pub applied_amount: Decimal,
    pub currency_code: String,
    pub currency_rate: Option<Decimal>,
    pub status: String,
    pub sub_total: Decimal,
    pub total_tax: Decimal,
    pub total: Decimal,
}

/// Represents an overpayment applied to an invoice
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Overpayment {
    #[serde(rename = "OverpaymentID")]
    pub overpayment_id: Uuid,
    pub overpayment_number: Option<String>,
    pub reference: Option<String>,
    pub applied_amount: Decimal,
    pub currency_code: String,
    pub currency_rate: Option<Decimal>,
    pub status: String,
    pub sub_total: Decimal,
    pub total_tax: Decimal,
    pub total: Decimal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Invoice {
    pub r#type: Type,
    pub contact: Contact,
    #[serde(rename = "DateString", with = "xero_date_format")]
    pub date: Date,
    #[serde(rename = "DueDateString", default, with = "xero_date_format_option")]
    pub due_date: Option<Date>,
    #[serde(default)]
    pub status: String,
    pub line_amount_types: LineAmountType,
    pub line_items: Vec<LineItem>,
    pub sub_total: Decimal,
    pub total_tax: Decimal,
    pub total: Decimal,
    pub total_discount: Option<Decimal>,
    #[serde(rename = "UpdatedDateUTC", with = "xero_datetime_format")]
    pub updated_date_utc: OffsetDateTime,
    pub currency_code: String,
    pub currency_rate: Option<Decimal>,
    #[serde(rename = "InvoiceID")]
    pub invoice_id: Uuid,
    #[serde(default)]
    pub invoice_number: Option<String>,
    pub reference: Option<String>,
    pub branding_theme_id: Option<Uuid>,
    pub url: Option<Url>,
    pub sent_to_contact: Option<bool>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub expected_payment_date: Option<Date>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub planned_payment_date: Option<Date>,
    #[serde(default)]
    pub has_attachments: bool,
    #[serde(rename = "RepeatingInvoiceID")]
    pub repeating_invoice_id: Option<Uuid>,
    #[serde(default)]
    pub payments: Option<Vec<Payment>>,
    #[serde(default)]
    pub credit_notes: Option<Vec<CreditNote>>,
    #[serde(default)]
    pub prepayments: Option<Vec<Prepayment>>,
    #[serde(default)]
    pub overpayments: Option<Vec<Overpayment>>,
    pub amount_due: Decimal,
    pub amount_paid: Decimal,
    #[serde(rename = "CISDeduction")]
    pub cis_deduction: Option<String>,
    #[serde(rename = "CISRate")]
    pub cis_rate: Option<Decimal>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub fully_paid_on_date: Option<Date>,
    #[serde(default)]
    pub amount_credited: Option<Decimal>,
    #[serde(flatten)]
    pub extra_fields: std::collections::HashMap<String, serde_json::Value>,
}

impl Invoice {
    /// Get the status of the invoice as an enum
    pub fn status_enum(&self) -> Option<Status> {
        match self.status.as_str() {
            "DRAFT" => Some(Status::Draft),
            "SUBMITTED" => Some(Status::Submitted),
            "DELETED" => Some(Status::Deleted),
            "AUTHORISED" => Some(Status::Authorised),
            "PAID" => Some(Status::Paid),
            "VOIDED" => Some(Status::Voided),
            _ => None,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ListResponse {
    pub invoices: Vec<Invoice>,
    pub status: String,
    pub id: Uuid,
    pub provider_name: String,
    #[serde(rename = "DateTimeUTC")]
    pub date_time_utc: Option<String>,
}

impl From<ListResponse> for Vec<Invoice> {
    fn from(response: ListResponse) -> Self {
        response.invoices
    }
}

#[derive(Debug, Serialize, Default)]
pub struct ListParameters {
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,

    /// Filter for invoices after a particular date
    #[serde(
        rename = "DateFrom",
        skip_serializing_if = "Option::is_none",
        with = "xero_date_format_option"
    )]
    pub date_from: Option<Date>,

    /// Filter for invoices before a particular date
    #[serde(
        rename = "DateTo",
        skip_serializing_if = "Option::is_none",
        with = "xero_date_format_option"
    )]
    pub date_to: Option<Date>,

    /// Filter for invoices due after a particular date
    #[serde(
        rename = "DueDateFrom",
        skip_serializing_if = "Option::is_none",
        with = "xero_date_format_option"
    )]
    pub due_date_from: Option<Date>,

    /// Filter for invoices due before a particular date
    #[serde(
        rename = "DueDateTo",
        skip_serializing_if = "Option::is_none",
        with = "xero_date_format_option"
    )]
    pub due_date_to: Option<Date>,

    /// Filter for invoices belonging to a particular contact
    #[serde(rename = "ContactID", skip_serializing_if = "Option::is_none")]
    pub contact_id: Option<Uuid>,

    /// Filter for invoices of a particular Status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,

    /// Pagination parameter (1-based)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,

    /// Order by any element
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,

    /// Filter by invoice number
    #[serde(rename = "InvoiceNumber", skip_serializing_if = "Option::is_none")]
    pub invoice_number: Option<String>,

    /// Include archived invoices
    #[serde(rename = "includeArchived", skip_serializing_if = "Option::is_none")]
    pub include_archived: Option<bool>,

    /// Only include invoices created by this app
    #[serde(rename = "createdByMyApp", skip_serializing_if = "Option::is_none")]
    pub created_by_my_app: Option<bool>,

    /// Filter by a comma-separated list of invoice IDs
    #[serde(rename = "IDs", skip_serializing_if = "Option::is_none")]
    pub ids: Option<String>,
}

impl ListParameters {
    /// Create a new builder for ListParameters
    #[must_use]
    pub fn builder() -> Self {
        Self::default()
    }

    /// Set the date_from filter
    #[must_use]
    pub fn with_date_from(mut self, date: Date) -> Self {
        self.date_from = Some(date);
        self
    }

    /// Set the date_to filter
    #[must_use]
    pub fn with_date_to(mut self, date: Date) -> Self {
        self.date_to = Some(date);
        self
    }

    /// Set the due_date_from filter
    #[must_use]
    pub fn with_due_date_from(mut self, date: Date) -> Self {
        self.due_date_from = Some(date);
        self
    }

    /// Set the due_date_to filter
    #[must_use]
    pub fn with_due_date_to(mut self, date: Date) -> Self {
        self.due_date_to = Some(date);
        self
    }

    /// Set the contact_id filter
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

    /// Set the invoice_number filter
    #[must_use]
    pub fn with_invoice_number(mut self, number: impl Into<String>) -> Self {
        self.invoice_number = Some(number.into());
        self
    }

    /// Set the include_archived filter
    #[must_use]
    pub fn with_include_archived(mut self, include: bool) -> Self {
        self.include_archived = Some(include);
        self
    }

    /// Set the created_by_my_app filter
    #[must_use]
    pub fn with_created_by_my_app(mut self, created_by_my_app: bool) -> Self {
        self.created_by_my_app = Some(created_by_my_app);
        self
    }

    /// Set the ids filter with a list of invoice IDs
    #[must_use]
    pub fn with_ids(mut self, ids: Vec<Uuid>) -> Self {
        let ids_str = ids
            .iter()
            .map(Uuid::to_string)
            .collect::<Vec<_>>()
            .join(",");
        self.ids = Some(ids_str);
        self
    }
}

#[derive(Default, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    #[serde(rename = "Type")]
    pub r#type: Type,
    pub contact: ContactIdentifier,
    pub line_items: Vec<line_item::Builder>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub date: Option<Date>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub due_date: Option<Date>,
    pub line_amount_types: Option<LineAmountType>,
    pub invoice_number: Option<String>,
    pub reference: Option<String>,
    #[serde(rename = "BrandingThemeID")]
    pub branding_theme_id: Option<Uuid>,
    pub url: Option<Url>,
    pub currency_code: Option<String>,
    pub currency_rate: Option<Decimal>,
    pub status: Option<Status>,
    pub sent_to_contact: Option<bool>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub expected_payment_date: Option<Date>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub planned_payment_date: Option<Date>,
    #[serde(rename = "InvoiceID", skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<Uuid>,
}

/// Request wrapper for invoices
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct InvoiceWrapper<'a> {
    pub invoices: Vec<&'a Builder>,
}

impl Builder {
    #[must_use]
    pub fn new(
        r#type: Type,
        contact: ContactIdentifier,
        line_items: Vec<line_item::Builder>,
    ) -> Self {
        Self {
            r#type,
            contact,
            line_items,
            ..Builder::default()
        }
    }
}

/// History record for an invoice
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

/// Attachment details for an invoice
#[derive(Debug, Deserialize, Serialize)]
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

/// Online invoice response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OnlineInvoice {
    pub online_invoice_url: String,
}

/// Online invoices response wrapper
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OnlineInvoices {
    pub online_invoices: Vec<OnlineInvoice>,
}

/// Implementation of `EntityEndpoint` for Invoice
impl EntityEndpoint<Invoice, ListParameters> for Invoice {
    fn endpoint() -> &'static str {
        ENDPOINT
    }

    async fn get(client: &Client, id: Uuid) -> Result<Invoice> {
        endpoint_utils::get::<Invoice, ListResponse>(client, ENDPOINT, id, "Invoice").await
    }

    async fn list(client: &Client, params: ListParameters) -> Result<Vec<Invoice>> {
        endpoint_utils::list::<Invoice, ListResponse, _>(client, ENDPOINT, &params).await
    }
}

/// Retrieve a list of invoices with filtering.
#[instrument(skip(client))]
pub async fn list(client: &Client, params: ListParameters) -> Result<Vec<Invoice>> {
    Invoice::list(client, params).await
}

/// Retrieve a list of all invoices without filtering.
#[instrument(skip(client))]
pub async fn list_all(client: &Client) -> Result<Vec<Invoice>> {
    Invoice::list(client, ListParameters::default()).await
}

/// Retrieve a single invoice by it's `invoice_id`.
#[instrument(skip(client))]
pub async fn get(client: &Client, invoice_id: Uuid) -> Result<Invoice> {
    Invoice::get(client, invoice_id).await
}

/// Create one or more invoices.
#[instrument(skip(client, invoice))]
pub async fn create(client: &Client, invoice: &Builder) -> Result<Invoice> {
    let request = InvoiceWrapper {
        invoices: vec![invoice],
    };

    let response: MutationResponse = client
        .put_endpoint(XeroEndpoint::Invoices, &request)
        .await?;

    // Extract invoice from response
    response
        .data
        .get_invoices()
        .and_then(|invoices| invoices.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Invoice".to_string(),
            url: XeroEndpoint::Invoices.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some("No invoice returned in response".to_string()),
        })
}

/// Update a specific invoice.
#[instrument(skip(client, invoice))]
pub async fn update(client: &Client, invoice_id: Uuid, invoice: &Builder) -> Result<Invoice> {
    let mut updatable_invoice = invoice.clone();
    updatable_invoice.invoice_id = Some(invoice_id);

    let request = InvoiceWrapper {
        invoices: vec![&updatable_invoice],
    };

    let endpoint = XeroEndpoint::Invoice(invoice_id);
    let response: MutationResponse = client.post_endpoint(endpoint.clone(), &request).await?;

    // Extract invoice from response
    response
        .data
        .get_invoices()
        .and_then(|invoices| invoices.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Invoice".to_string(),
            url: endpoint.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some(format!("Invoice with ID {invoice_id} not found")),
        })
}

/// Update or create one or more invoices.
#[instrument(skip(client, invoice))]
pub async fn update_or_create(client: &Client, invoice: &Builder) -> Result<Invoice> {
    let request = InvoiceWrapper {
        invoices: vec![invoice],
    };

    let response: MutationResponse = client
        .post_endpoint(XeroEndpoint::Invoices, &request)
        .await?;

    // Extract invoice from response
    response
        .data
        .get_invoices()
        .and_then(|invoices| invoices.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Invoice".to_string(),
            url: XeroEndpoint::Invoices.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some("No invoice returned in response".to_string()),
        })
}

/// Retrieve a invoice as a PDF file.
#[instrument(skip(client))]
pub async fn get_pdf(client: &Client, invoice_id: Uuid) -> Result<Vec<u8>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Invoices".to_string(),
        invoice_id.to_string(),
        "pdf".to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::GET, url)
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(Error::NotFound {
            entity: "Invoice PDF".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to retrieve PDF for invoice with ID {invoice_id}"
            )),
        })
    }
}

/// Get the online invoice URL
pub async fn get_online_invoice(client: &Client, invoice_id: Uuid) -> Result<String> {
    let endpoint = XeroEndpoint::from_string(format!(
        "https://api.xero.com/api.xro/2.0/Invoices/{}/OnlineInvoice",
        invoice_id
    ));
    let empty_tuple = ();
    let response: OnlineInvoices = client.get_endpoint(endpoint, &empty_tuple).await?;
    Ok(response.online_invoices[0].online_invoice_url.clone())
}

/// Email the invoice to the contact
#[instrument(skip(client))]
pub async fn email(client: &Client, invoice_id: Uuid) -> Result<()> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Invoices".to_string(),
        invoice_id.to_string(),
        "Email".to_string(),
    ]);

    // Empty request body
    let empty_request = serde_json::json!({});

    let _: serde_json::Value = client.post_endpoint(endpoint, &empty_request).await?;

    Ok(())
}

/// Get history records for an invoice
pub async fn get_history(client: &Client, invoice_id: Uuid) -> Result<Vec<HistoryRecord>> {
    let endpoint = XeroEndpoint::from_string(format!(
        "https://api.xero.com/api.xro/2.0/Invoices/{}/history",
        invoice_id
    ));
    let empty_tuple = ();
    let response: HistoryRecords = client.get_endpoint(endpoint, &empty_tuple).await?;
    Ok(response.history_records)
}

/// Create a history record for a specific invoice.
#[instrument(skip(client))]
pub async fn create_history(
    client: &Client,
    invoice_id: Uuid,
    details: &str,
) -> Result<Vec<HistoryRecord>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Invoices".to_string(),
        invoice_id.to_string(),
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

/// List attachments for an invoice
pub async fn list_attachments(client: &Client, invoice_id: Uuid) -> Result<Vec<Attachment>> {
    let endpoint = XeroEndpoint::from_string(format!(
        "https://api.xero.com/api.xro/2.0/Invoices/{}/Attachments",
        invoice_id
    ));
    let empty_tuple = ();
    let response: Attachments = client.get_endpoint(endpoint, &empty_tuple).await?;
    Ok(response.attachments)
}

/// Get a specific attachment by ID.
#[instrument(skip(client))]
pub async fn get_attachment(
    client: &Client,
    invoice_id: Uuid,
    attachment_id: Uuid,
) -> Result<Vec<u8>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Invoices".to_string(),
        invoice_id.to_string(),
        "Attachments".to_string(),
        attachment_id.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::GET, url)
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(Error::NotFound {
            entity: "Invoice Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to retrieve attachment for invoice with ID {invoice_id}"
            )),
        })
    }
}

/// Get an attachment by filename.
#[instrument(skip(client))]
pub async fn get_attachment_by_filename(
    client: &Client,
    invoice_id: Uuid,
    filename: &str,
) -> Result<Vec<u8>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Invoices".to_string(),
        invoice_id.to_string(),
        "Attachments".to_string(),
        filename.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::GET, url)
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(Error::NotFound {
            entity: "Invoice Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to retrieve attachment {} for invoice with ID {invoice_id}",
                filename
            )),
        })
    }
}

/// Upload an attachment to an invoice.
#[instrument(skip(client, attachment_content))]
pub async fn upload_attachment(
    client: &Client,
    invoice_id: Uuid,
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
        "Invoices".to_string(),
        invoice_id.to_string(),
        "Attachments".to_string(),
        filename.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::PUT, url)
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
                entity: "Invoice Attachment".to_string(),
                url: endpoint.to_string(),
                status_code: status,
                response_body: Some("No attachment was returned after upload".to_string()),
            })
    } else {
        Err(Error::NotFound {
            entity: "Invoice Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to upload attachment for invoice with ID {invoice_id}"
            )),
        })
    }
}

/// Update an existing attachment.
#[instrument(skip(client, attachment_content))]
pub async fn update_attachment(
    client: &Client,
    invoice_id: Uuid,
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
        "Invoices".to_string(),
        invoice_id.to_string(),
        "Attachments".to_string(),
        filename.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::POST, url)
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
                entity: "Invoice Attachment".to_string(),
                url: endpoint.to_string(),
                status_code: status,
                response_body: Some("No attachment was returned after update".to_string()),
            })
    } else {
        Err(Error::NotFound {
            entity: "Invoice Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to update attachment for invoice with ID {invoice_id}"
            )),
        })
    }
}

// Keep post_attachment as an alias for upload_attachment for backward compatibility
/// Post an attachment to an invoice.
/// This function is an alias for upload_attachment and is kept for backward compatibility.
#[instrument(skip(client, attachment_content))]
pub async fn post_attachment(
    client: &Client,
    invoice_id: Uuid,
    attachment_filename: String,
    attachment_content: &[u8],
) -> Result<Value> {
    let attachment =
        upload_attachment(client, invoice_id, &attachment_filename, attachment_content).await?;

    // Convert the Attachment to a Value for backward compatibility
    Ok(serde_json::to_value(attachment)?)
}
