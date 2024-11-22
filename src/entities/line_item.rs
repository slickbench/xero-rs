use rust_decimal::Decimal;
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
    pub quantity: Decimal,
    pub unit_amount: Decimal,
    pub item_code: Option<String>,
    pub account_code: Option<String>,
    #[serde(rename = "LineItemID")]
    pub line_item_id: Uuid,
    pub tax_type: String,
    pub tax_amount: Decimal,
    pub line_amount: Decimal,
    pub discount_rate: Option<Decimal>,
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
        builder.line_item_id = Some(self.line_item_id);

        builder
    }
}

#[derive(Default, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    pub description: String,
    pub quantity: Decimal,
    pub unit_amount: Decimal,
    pub item_code: Option<String>,
    pub account_code: Option<String>,
    pub tax_type: String,
    pub discount_rate: Option<Decimal>,
    // tracking
    #[serde(rename = "LineItemID")]
    pub line_item_id: Option<Uuid>,
}

impl Builder {
    #[must_use]
    pub fn new(description: String, quantity: Decimal, unit_amount: Decimal) -> Self {
        Builder {
            description,
            quantity,
            unit_amount,
            ..Self::default()
        }
    }
}
