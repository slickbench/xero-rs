use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LineAmountType {
    #[serde(alias = "EXCLUSIVE")]
    Exclusive,
    #[serde(alias = "INCLUSIVE")]
    Inclusive,
    #[serde(alias = "NOTAX")]
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
