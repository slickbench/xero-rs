use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LineAmountType {
    #[serde(alias = "EXCLUSIVE")]
    Exclusive,
    #[serde(alias = "INCLUSIVE")]
    Inclusive,
    #[serde(alias = "NOTAX")]
    NoTax,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LineItem {
    pub description: String,
    pub quantity: f64,
    pub unit_amount: f64,
    pub item_code: Option<String>,
    pub account_code: Option<String>,
    #[serde(rename = "LineItemID")]
    pub line_item_id: Uuid,
    pub tax_type: String,
    pub tax_amount: f64,
    pub line_amount: f64,
    pub discount_rate: Option<f64>,
    // tracking
}

impl LineItem {
    #[must_use]
    pub fn into_builder(self) -> Builder {
        let mut builder = Builder::new(self.description, self.quantity, self.unit_amount);
        builder.item_code = self.item_code;
        builder.account_code = self.account_code;
        builder.tax_type = self.tax_type;
        builder.discount_rate = self.discount_rate;
        builder.line_item_id = self.line_item_id;

        builder
    }
}

#[derive(Default, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    description: String,
    quantity: f64,
    unit_amount: f64,
    item_code: Option<String>,
    account_code: Option<String>,
    tax_type: String,
    discount_rate: Option<f64>,
    // tracking
    #[serde(rename = "LineItemID")]
    line_item_id: Uuid,
}

impl Builder {
    #[must_use]
    pub fn new(description: String, quantity: f64, unit_amount: f64) -> Self {
        Builder {
            description,
            quantity,
            unit_amount,
            ..Self::default()
        }
    }
}
