use std::{path::Path, ffi::OsStr};

use chrono::NaiveDateTime;
use reqwest::Method;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;
use uuid::Uuid;

use crate::{
    contact::Contact,
    endpoints::XeroEndpoint,
    error::{Error, Result},
    line_item::{LineAmountType, LineItem},
    Client,
};

use super::{line_item};

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
pub(crate) struct ListResponse {
    pub invoices: Vec<Invoice>,
}

#[derive(Debug, Serialize, Default)]
pub struct ListParameters {
    pub r#where: Option<String>,
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

/// Post an attachment to an invoice.
#[instrument(skip(client, attachment_content))]
pub async fn post_attachment(
    client: &Client,
    invoice_id: Uuid,
    attachment_filename: String,
    attachment_content: &[u8],
) -> Result<Value> {
    // Define constants first
    const MAX_ATTACHMENT_SIZE: usize = 25 * 1024 * 1024; // 25 MB

    // 1. Check if filename is valid
    if attachment_filename.is_empty() {
        return Err(Error::InvalidFilename);
    }

    // 2. Determine content type from filename extension
    let ext = Path::new(&attachment_filename)
        .extension()
        .and_then(OsStr::to_str);

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
        attachment_filename.clone()
    ]);
    
    // Make the request
    let url = endpoint.to_url()?;
    let response = client
        .build_request(Method::POST, url)
        .header(reqwest::header::CONTENT_TYPE, content_type)
        .header(reqwest::header::CONTENT_LENGTH, attachment_content.len())
        .body(attachment_content.to_vec())
        .send()
        .await?;

    // Parse and return the response
    Ok(response.json().await?)
}
