use time::{Date, OffsetDateTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    contact::Contact,
    line_item::{self, LineAmountType, LineItem},
    utils::date_format::{xero_date_format, xero_date_format_option, xero_datetime_format},
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
    #[serde(with = "xero_date_format")]
    pub date: Date,
    #[serde(default, with = "xero_date_format_option")]
    pub delivery_date: Option<Date>,
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
    #[serde(default, with = "xero_date_format_option")]
    pub expected_arrival_date: Option<Date>,
    #[serde(rename = "PurchaseOrderID")]
    pub purchase_order_id: Uuid,
    pub currency_rate: Option<Decimal>,
    pub sub_total: Decimal,
    pub total_tax: Decimal,
    pub total: Decimal,
    pub total_discount: Option<Decimal>,
    pub has_attachments: Option<bool>,
    #[serde(rename = "UpdatedDateUTC", with = "xero_datetime_format")]
    pub updated_date_utc: OffsetDateTime,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ListResponse {
    pub purchase_orders: Vec<PurchaseOrder>,
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
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    pub contact: ContactIdentifier,
    pub line_items: Vec<line_item::Builder>,
    #[serde(with = "xero_date_format_option", skip_serializing_if = "Option::is_none")]
    pub date: Option<Date>,
    #[serde(with = "xero_date_format_option", skip_serializing_if = "Option::is_none")]
    pub delivery_date: Option<Date>,
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
    #[serde(with = "xero_date_format_option", skip_serializing_if = "Option::is_none")]
    pub expected_arrival_date: Option<Date>,
    #[serde(rename = "PurchaseOrderID")]
    pub purchase_order_id: Option<Uuid>,
}

impl Builder {
    #[must_use]
    pub fn new(contact: ContactIdentifier, line_items: Vec<line_item::Builder>) -> Self {
        Self {
            contact,
            line_items,
            ..Default::default()
        }
    }
}
