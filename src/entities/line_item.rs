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
    #[serde(rename = "LineItemID")]
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_amount: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Decimal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<Decimal>,
    #[serde(default)]
    pub tracking: Vec<serde_json::Value>,
    #[serde(default)]
    pub validation_errors: Vec<serde_json::Value>,
}

impl LineItem {
    #[must_use]
    pub fn into_builder(self) -> Builder {
        let mut builder = Builder::new(self.description, self.quantity, self.unit_amount);
        builder.item_code = self.item_code;
        builder.account_code = self.account_code;
        builder.tax_type = self.tax_type;
        builder.discount_rate = self.discount_rate;
        builder.discount_amount = self.discount_amount;
        builder.id = Some(self.id);

        builder
    }
}

#[derive(Default, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    #[serde(rename = "LineItemID")]
    pub id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_amount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_rate: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discount_amount: Option<Decimal>,
}

impl Builder {
    #[must_use]
    pub fn new(
        description: Option<String>,
        quantity: Option<Decimal>,
        unit_amount: Option<Decimal>,
    ) -> Self {
        Builder {
            description,
            quantity,
            unit_amount,
            ..Self::default()
        }
    }
}
