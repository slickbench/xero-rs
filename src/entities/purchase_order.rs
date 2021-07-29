use std::str::FromStr;

use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    contact::Contact,
    error::{Error, Result},
    line_item::{self, LineAmountType, LineItem},
    Client, MutationResponse,
};

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/PurchaseOrders/";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Draft,
    Submitted,
    Authorised,
    Billed,
    Deleted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PurchaseOrder {
    pub contact: Contact,
    pub date: String,
    pub delivery_date: Option<String>,
    pub line_amount_types: LineAmountType,
    pub purchase_order_number: String,
    pub reference: Option<String>,
    #[serde(default)]
    pub line_items: Vec<LineItem>,
    pub branding_theme_id: Option<Uuid>,
    pub currency_code: String,
    pub status: Status,
    pub sent_to_contact: Option<bool>,
    // pub delivery_address: Option<String>,
    pub attention_to: Option<String>,
    pub telephone: Option<String>,
    pub delivery_instructions: Option<String>,
    pub expected_arrival_date: Option<String>,
    #[serde(rename = "PurchaseOrderID")]
    pub purchase_order_id: Uuid,
    pub currency_rate: Option<Decimal>,
    pub sub_total: Decimal,
    pub total_tax: Decimal,
    pub total: Decimal,
    pub total_discount: Option<Decimal>,
    pub has_attachments: Option<bool>,
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListResponse {
    purchase_orders: Vec<PurchaseOrder>,
}

/// Retrieve a list of purchase orders.
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<PurchaseOrder>> {
    let response: ListResponse = client.get(ENDPOINT, Vec::<String>::default()).await?;
    Ok(response.purchase_orders)
}

/// Retrieve a single purchase order by it's `purchase_order_id`.
#[instrument(skip(client))]
pub async fn get(client: &Client, purchase_order_id: Uuid) -> Result<PurchaseOrder> {
    let endpoint = Url::from_str(ENDPOINT)
        .and_then(|endpoint| endpoint.join(&purchase_order_id.to_string()))
        .map_err(|_| Error::InvalidEndpoint)?;
    let response: ListResponse = client.get(endpoint, Vec::<String>::default()).await?;
    response
        .purchase_orders
        .into_iter()
        .next()
        .ok_or(Error::NotFound)
}

#[derive(Debug, Serialize)]
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

#[derive(Default, Debug, Serialize)]
pub struct Builder {
    pub contact: ContactIdentifier,
    pub line_items: Vec<line_item::Builder>,
    pub date: Option<NaiveDateTime>,
    pub delivery_date: Option<NaiveDateTime>,
    pub line_amount_types: Option<LineAmountType>,
    pub purchase_order_number: Option<String>,
    pub reference: Option<String>,
    #[serde(rename = "BrandingThemeID")]
    pub branding_theme_id: Option<Uuid>,
    pub currency_code: Option<String>,
    pub status: Option<Status>,
    pub sent_to_contact: Option<bool>,
    pub delivery_address: Option<String>,
    pub attention_to: Option<String>,
    pub telephone: Option<String>,
    pub delivery_instructions: Option<String>,
    pub expected_arrival_date: Option<NaiveDateTime>,
    #[serde(rename = "PurchaseOrderID")]
    pub purchase_order_id: Option<Uuid>,
}

impl Builder {
    #[must_use]
    pub fn new(contact: ContactIdentifier, line_items: Vec<line_item::Builder>) -> Self {
        Self {
            contact,
            line_items,
            ..Builder::default()
        }
    }
}

#[instrument(skip(client))]
pub async fn create(client: &Client, purchase_order: &Builder) -> Result<PurchaseOrder> {
    let result: MutationResponse = client.put(ENDPOINT, purchase_order).await?;
    result
        .data
        .get_purchase_orders()
        .and_then(|po| po.into_iter().next())
        .ok_or(Error::NotFound)
}
