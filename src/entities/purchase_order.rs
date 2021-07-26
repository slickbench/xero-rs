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

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/PurchaseOrders/";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Draft,
    Submitted,
    Authorised,
    Billed,
    Deleted,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PurchaseOrder {
    pub contact: Contact,
    pub date: String,
    pub delivery_date: Option<String>,
    pub line_amount_types: LineAmountType,
    pub purchase_order_number: String,
    pub reference: String,
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
    pub currency_rate: Option<f64>,
    pub sub_total: f64,
    pub total_tax: f64,
    pub total: f64,
    pub total_discount: Option<f64>,
    pub has_attachments: bool,
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
