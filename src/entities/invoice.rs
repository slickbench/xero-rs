use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{error::Result, Client};

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/Invoices";

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
    quantity: i64,
    unit_amount: f64,
    item_code: String,
    account_code: String,
    line_item_id: Uuid,
    tax_type: String,
    tax_amount: f64,
    line_amount: f64,
    discount_rate: f64,
    // tracking
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Invoice {
    r#type: Type,
    // contact
    #[serde(rename = "DateString")]
    date: NaiveDateTime,
    #[serde(rename = "DueDateString")]
    due_date: NaiveDateTime,
    status: Status,
    line_amount_types: LineAmountType,
    line_items: Vec<LineItem>,
    sub_total: f64,
    total_tax: f64,
    total: f64,
    total_discount: Option<f64>,
    #[serde(rename = "UpdatedDateUTC")]
    updated_date_utc: String,
    currency_code: String,
    // currency_rate
    #[serde(rename = "InvoiceID")]
    invoice_id: Uuid,
    invoice_number: String,
    reference: Option<String>,
    // branding_theme_id
    url: Option<Url>,
    sent_to_contact: Option<bool>,
    // expected_payment_date
    // planned_payment_date
    has_attachments: bool,
    #[serde(rename = "RepeatingInvoiceID")]
    repeating_invoice_id: Option<Uuid>,
    // payments
    // credit_notes
    // prepayments
    // overpayments
    amount_due: f64,
    amount_paid: f64,
    // CISDeduction
    // fully_paid_on_date
    amount_credited: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InvoiceListResponse {
    invoices: Vec<Invoice>,
}

/// Retrieve a list of invoices.
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<Invoice>> {
    let response: InvoiceListResponse = client.get(ENDPOINT, Vec::<String>::default()).await?;
    Ok(response.invoices)
}
