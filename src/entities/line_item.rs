use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum LineAmountType {
    #[serde(alias = "NONE")]
    None,
    #[serde(alias = "EXCLUSIVE")]
    Exclusive,
    #[serde(alias = "INCLUSIVE")]
    Inclusive,
    #[serde(alias = "NOTAX")]
    NoTax,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ItemSummary {
    #[serde(rename = "ItemID")]
    pub item_id: Uuid,
    pub name: String,
    pub code: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TrackingSummary {
    pub name: String,
    pub option: String,
    #[serde(rename = "TrackingCategoryID")]
    pub tracking_category_id: Uuid,
    #[serde(rename = "TrackingOptionID")]
    pub tracking_option_id: Option<Uuid>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LineItem {
    #[serde(rename = "LineItemID")]
    pub line_item_id: Uuid,
    pub description: String,
    pub quantity: Option<Decimal>,
    pub unit_amount: Option<Decimal>,
    pub item_code: Option<String>,
    pub account_code: Option<String>,
    #[serde(rename = "AccountID")]
    pub account_id: Option<Uuid>,
    pub item: Option<ItemSummary>,
    pub tracking: Vec<TrackingSummary>,
    pub tax_type: Option<String>,
    pub tax_amount: Decimal,
    pub line_amount: Option<Decimal>,
    pub discount_rate: Option<Decimal>,
    pub discount_amount: Option<Decimal>
}

impl LineItem {
    #[must_use]
    pub fn into_builder(self) -> Builder {
        let mut builder = Builder::new();
        builder.description = Some(self.description);
        builder.quantity = self.quantity;
        builder.unit_amount = self.unit_amount;
        builder.item_code = self.item_code;
        builder.account_code = self.account_code;
        builder.line_item_id = Some(self.line_item_id);
        builder.tax_type = self.tax_type;
        builder.line_amount = self.line_amount;
        builder.discount_rate = self.discount_rate;
        builder.discount_amount = self.discount_amount;
        builder.tracking = Some(self.tracking);

        builder
    }
}

#[derive(Default, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    pub description: Option<String>,
    pub quantity: Option<Decimal>,
    pub unit_amount: Option<Decimal>,
    pub item_code: Option<String>,
    pub account_code: Option<String>,
    #[serde(rename = "LineItemID")]
    pub line_item_id: Option<Uuid>,
    pub tax_type: Option<String>,
    pub line_amount: Option<Decimal>,
    pub discount_rate: Option<Decimal>,
    pub discount_amount: Option<Decimal>,
    pub tracking: Option<Vec<TrackingSummary>>,
}

impl Builder {
    #[must_use]
    pub fn new() -> Self {
        Builder {
            ..Self::default()
        }
    }
}
