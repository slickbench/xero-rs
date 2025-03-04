use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    contact::Contact,
    entities::{EntityEndpoint, endpoint_utils},
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
    pub title: String,
    pub summary: Option<String>,
    pub terms: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct ListResponse {
    quotes: Vec<Quote>,
}

impl From<ListResponse> for Vec<Quote> {
    fn from(response: ListResponse) -> Self {
        response.quotes
    }
}

/// Empty parameters struct for quote listing (could be extended with filters if needed)
#[derive(Debug, Serialize, Default)]
pub struct ListParameters {}

/// Implementation of EntityEndpoint for Quote
impl EntityEndpoint<Quote, ListParameters> for Quote {
    fn endpoint() -> &'static str {
        ENDPOINT
    }
    
    async fn get(client: &Client, id: Uuid) -> Result<Quote> {
        endpoint_utils::get::<Quote, ListResponse>(client, ENDPOINT, id, "Quote").await
    }
    
    async fn list(client: &Client, params: ListParameters) -> Result<Vec<Quote>> {
        endpoint_utils::list::<Quote, ListResponse, _>(client, ENDPOINT, params).await
    }
}

/// Retrieve a list of quotes.
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<Quote>> {
    Quote::list(client, ListParameters::default()).await
}

/// Retrieve a single quote by it's `quote_id`.
#[instrument(skip(client))]
pub async fn get(client: &Client, quote_id: Uuid) -> Result<Quote> {
    Quote::get(client, quote_id).await
}
