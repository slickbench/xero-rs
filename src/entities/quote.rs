use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    contact::Contact,
    error::{Error, Result},
    line_item::{LineAmountType, LineItem},
    Client,
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
    pub sub_total: f64,
    pub total_tax: f64,
    pub total: f64,
    pub total_discount: Option<f64>,
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: String,
    pub currency_code: String,
    pub currency_rate: Option<f64>,
    #[serde(rename = "QuoteID")]
    pub quote_id: Uuid,
    pub quote_number: String,
    pub reference: Option<String>,
    pub branding_theme_id: Option<Uuid>,
    pub title: String,
    pub summary: Option<String>,
    pub terms: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListResponse {
    quotes: Vec<Quote>,
}

/// Retrieve a list of quotes.
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<Quote>> {
    let response: ListResponse = client.get(ENDPOINT, Vec::<String>::default()).await?;
    Ok(response.quotes)
}

/// Retrieve a single quote by it's `quote_id`.
#[instrument(skip(client))]
pub async fn get(client: &Client, quote_id: Uuid) -> Result<Quote> {
    let endpoint = Url::from_str(ENDPOINT)
        .and_then(|endpoint| endpoint.join(&quote_id.to_string()))
        .map_err(|_| Error::InvalidEndpoint)?;
    let response: ListResponse = client.get(endpoint, Vec::<String>::default()).await?;
    response.quotes.into_iter().next().ok_or(Error::NotFound)
}
