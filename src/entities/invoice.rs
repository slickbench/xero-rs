use std::str::FromStr;

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    contact::Contact,
    error::{Error, Result},
    line_item::{LineAmountType, LineItem},
    Client,
};

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/Invoices/";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Type {
    #[serde(rename = "ACCPAY")]
    AccountsPayable,

    #[serde(rename = "ACCREC")]
    AccountsReceivable,
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
    #[serde(rename = "DueDateString")]
    pub due_date: NaiveDateTime,
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
    pub amount_due: Decimal,
    pub amount_paid: Decimal,
    #[serde(rename = "CISDeduction")]
    pub cis_deduction: Option<String>,
    pub fully_paid_on_date: Option<String>,
    pub amount_credited: Decimal,
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

/// Retrieve a single invoice by it's `invoice_id`.
#[instrument(skip(client))]
pub async fn get(client: &Client, invoice_id: Uuid) -> Result<Invoice> {
    let endpoint = Url::from_str(ENDPOINT)
        .and_then(|endpoint| endpoint.join(&invoice_id.to_string()))
        .map_err(|_| Error::InvalidEndpoint)?;
    let response: ListResponse = client.get(endpoint, Vec::<String>::default()).await?;
    response.invoices.into_iter().next().ok_or(Error::NotFound)
}
