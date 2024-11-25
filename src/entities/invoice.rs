use std::{path::Path, str::FromStr};

use chrono::NaiveDateTime;
use reqwest::Method;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;

use crate::{
    contact::Contact,
    error::{Error, Result},
    line_item::{LineAmountType, LineItem},
    Client,
};

use super::{line_item, MutationResponse};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Invoice {
    pub r#type: Type,
    pub contact: Contact,
    #[serde(rename = "DateString")]
    pub date: NaiveDateTime,
    #[serde(rename = "DueDateString", default)]
    pub due_date: Option<NaiveDateTime>,
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
    #[serde(rename = "InvoiceID")]
    pub invoice_id: Uuid,
    #[serde(default)]
    pub invoice_number: Option<String>,
    pub reference: Option<String>,
    pub branding_theme_id: Option<Uuid>,
    pub url: Option<Url>,
    pub sent_to_contact: Option<bool>,
    pub expected_payment_date: Option<String>,
    pub planned_payment_date: Option<String>,
    #[serde(default)]
    pub has_attachments: bool,
    #[serde(rename = "RepeatingInvoiceID")]
    pub repeating_invoice_id: Option<Uuid>,
    // payments
    // credit_notes
    // prepayments
    // overpayments
    pub amount_due: Decimal,
    pub amount_paid: Decimal,
    #[serde(rename = "CISDeduction")]
    pub cis_deduction: Option<String>,
    pub fully_paid_on_date: Option<String>,
    #[serde(default)]
    pub amount_credited: Option<Decimal>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListResponse {
    invoices: Vec<Invoice>,
}

#[derive(Debug, Serialize, Default)]
pub struct ListParameters {
    pub r#where: Option<String>,
}

/// Retrieve a list of invoices.
#[instrument(skip(client))]
pub async fn list(client: &Client, parameters: ListParameters) -> Result<Vec<Invoice>> {
    let response: ListResponse = client.get(ENDPOINT, parameters).await?;
    Ok(response.invoices)
}

/// Retrieve a single invoice by it's `invoice_id`.
#[instrument(skip(client))]
pub async fn get(client: &Client, invoice_id: Uuid) -> Result<Invoice> {
    let endpoint = Url::from_str(ENDPOINT)
        .and_then(|endpoint| endpoint.join(&invoice_id.to_string()))
        .map_err(|_| Error::InvalidEndpoint)?;
    let response: ListResponse = client.get(endpoint, Vec::<String>::default()).await?;
    response.invoices.into_iter().next().ok_or(Error::NotFound)
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub enum ContactIdentifier {
    #[serde(rename = "ContactID")]
    ID(Uuid),
    #[serde(rename = "ContactNumber")]
    Number(String),
}

impl Default for ContactIdentifier {
    fn default() -> Self {
        Self::ID(Uuid::new_v4())
    }
}

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    #[serde(rename = "Type")]
    pub r#type: Type,
    pub contact: ContactIdentifier,
    pub line_items: Vec<line_item::Builder>,
    pub date: Option<NaiveDateTime>,
    pub due_date: Option<NaiveDateTime>,
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
    pub expected_payment_date: Option<NaiveDateTime>,
    pub planned_payment_date: Option<NaiveDateTime>,
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

#[instrument(skip(client))]
pub async fn create(client: &Client, invoice: &Builder) -> Result<Invoice> {
    let result: MutationResponse = client.put(ENDPOINT, invoice).await?;
    result
        .data
        .get_invoices()
        .and_then(|inv| inv.into_iter().next())
        .ok_or(Error::NotFound)
}

pub async fn post_attachment(
    client: &Client,
    invoice_id: Uuid,
    attachment_filename: String,
    attachment_content: &[u8],
) -> Result<Value> {
    // 1. Validate the filename for invalid characters
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];
    if attachment_filename
        .chars()
        .any(|c| invalid_chars.contains(&c))
    {
        return Err(Error::InvalidFilename);
    }

    // 2. Determine Content-Type based on file extension
    let extension = Path::new(&attachment_filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    let content_type = match extension.as_deref() {
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        // Add more mappings as needed
        _ => "application/octet-stream", // Default fallback
    };

    // 3. Validate attachment size (up to 25 MB)
    const MAX_ATTACHMENT_SIZE: usize = 25 * 1024 * 1024; // 25 MB
    if attachment_content.len() > MAX_ATTACHMENT_SIZE {
        return Err(Error::AttachmentTooLarge);
    }

    // 4. Construct the URL
    let endpoint_url = Url::from_str(ENDPOINT)
        .map_err(|_| Error::InvalidEndpoint)?
        .join(&format!("{}/", invoice_id.to_string()))
        .map_err(|_| Error::InvalidEndpoint)?
        .join("Attachments/")
        .map_err(|_| Error::InvalidEndpoint)?
        .join(&attachment_filename)
        .map_err(|_| Error::InvalidEndpoint)?;

    info!("Posting attachment to {}", endpoint_url);

    // 5. Build and send the POST request
    let response = client
        .build_request(Method::POST, endpoint_url)
        .header(reqwest::header::CONTENT_TYPE, content_type)
        .header(reqwest::header::CONTENT_LENGTH, attachment_content.len())
        .body(attachment_content.to_vec())
        .send()
        .await
        .map_err(Error::Request)?;

    // Optional: Handle and log the response
    if response.status().is_success() {
        info!("Attachment uploaded successfully.");
    } else {
        info!("Failed to upload attachment. Status: {}", response.status());
    }

    Ok(response.json::<Value>().await.unwrap())
}
