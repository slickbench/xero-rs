use std::str::FromStr;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    contact::Contact,
    error::{Error, Result},
    Client,
};

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/Invoices/";

#[derive(Debug, Serialize, Deserialize)]
pub enum Type {
    #[serde(rename = "ACCPAY")]
    AccountsPayable,

    #[serde(rename = "ACCREC")]
    AccountsReceivable,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Draft,
    Submitted,
    Deleted,
    Authorised,
    Paid,
    Voided,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LineAmountType {
    Exclusive,
    Inclusive,
    NoTax,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LineItem {
    description: String,
    quantity: f64,
    unit_amount: f64,
    item_code: Option<String>,
    account_code: String,
    #[serde(rename = "LineItemID")]
    line_item_id: Uuid,
    tax_type: String,
    tax_amount: f64,
    line_amount: f64,
    discount_rate: Option<f64>,
    // tracking
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Invoice {
    pub r#type: Type,
    pub contact: Contact,
    #[serde(rename = "DateString")]
    pub date: NaiveDateTime,
    #[serde(rename = "DueDateString")]
    pub due_date: NaiveDateTime,
    pub status: Status,
    pub line_amount_types: LineAmountType,
    pub line_items: Vec<LineItem>,
    pub sub_total: f64,
    pub total_tax: f64,
    pub total: f64,
    pub total_discount: Option<f64>,
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: String,
    pub currency_code: String,
    pub currency_rate: Option<f64>,
    #[serde(rename = "InvoiceID")]
    pub invoice_id: Uuid,
    pub invoice_number: String,
    pub reference: Option<String>,
    pub branding_theme_id: Option<Uuid>,
    pub url: Option<Url>,
    pub sent_to_contact: Option<bool>,
    pub expected_payment_date: Option<String>,
    pub planned_payment_date: Option<String>,
    pub has_attachments: bool,
    #[serde(rename = "RepeatingInvoiceID")]
    pub repeating_invoice_id: Option<Uuid>,
    // payments
    // credit_notes
    // prepayments
    // overpayments
    pub amount_due: f64,
    pub amount_paid: f64,
    #[serde(rename = "CISDeduction")]
    pub cis_deduction: Option<String>,
    pub fully_paid_on_date: Option<String>,
    pub amount_credited: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListResponse {
    invoices: Vec<Invoice>,
}

/// Retrieve a list of invoices.
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<Invoice>> {
    let response: ListResponse = client.get(ENDPOINT, Vec::<String>::default()).await?;
    Ok(response.invoices)
}

/// Retrieve a single invoice by an `invoice_id`.
#[instrument(skip(client))]
pub async fn get(client: &Client, invoice_id: Uuid) -> Result<Invoice> {
    let endpoint = Url::from_str(ENDPOINT)
        .and_then(|endpoint| endpoint.join(&invoice_id.to_string()))
        .map_err(|_| Error::InvalidEndpoint)?;
    let response: ListResponse = client.get(endpoint, Vec::<String>::default()).await?;
    response.invoices.into_iter().next().ok_or(Error::NotFound)
}
