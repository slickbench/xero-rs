use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::{
    contact::{Contact, ContactIdentifier},
    entities::{invoice, line_item},
    error::ValidationError,
    line_item::{LineAmountType, LineItem},
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
    #[serde(
        default,
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivery_date: Option<Date>,
    pub line_amount_types: LineAmountType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_order_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default)]
    pub line_items: Vec<LineItem>,
    #[serde(rename = "BrandingThemeID", skip_serializing_if = "Option::is_none")]
    pub branding_theme_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_code: Option<String>,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_to_contact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telephone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_instructions: Option<String>,
    #[serde(
        default,
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub expected_arrival_date: Option<Date>,
    #[serde(rename = "PurchaseOrderID")]
    pub purchase_order_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_rate: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_total: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tax: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_discount: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_attachments: Option<bool>,
    #[serde(rename = "UpdatedDateUTC", with = "xero_datetime_format")]
    pub updated_date_utc: OffsetDateTime,
    #[serde(
        rename = "StatusAttributeString",
        skip_serializing_if = "Option::is_none"
    )]
    pub status_attribute_string: Option<String>,
    #[serde(
        default,
        rename = "ValidationErrors",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub validation_errors: Vec<ValidationError>,
    #[serde(default, rename = "Warnings", skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<ValidationError>,
    #[serde(default, rename = "Attachments", skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<invoice::Attachment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ListResponse {
    pub purchase_orders: Vec<PurchaseOrder>,
}

#[derive(Default, Debug, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<ContactIdentifier>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub line_items: Vec<line_item::Builder>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub date: Option<Date>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub delivery_date: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_amount_types: Option<LineAmountType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_order_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(rename = "BrandingThemeID", skip_serializing_if = "Option::is_none")]
    pub branding_theme_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sent_to_contact: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attention_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telephone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_instructions: Option<String>,
    #[serde(
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub expected_arrival_date: Option<Date>,
    #[serde(rename = "PurchaseOrderID", skip_serializing_if = "Option::is_none")]
    pub purchase_order_id: Option<Uuid>,
}

impl Builder {
    #[must_use]
    pub fn new(contact: ContactIdentifier, line_items: Vec<line_item::Builder>) -> Self {
        Self {
            contact: Some(contact),
            line_items,
            ..Default::default()
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PurchaseOrdersRequest<'a> {
    pub purchase_orders: Vec<&'a Builder>,
}

impl<'a> PurchaseOrdersRequest<'a> {
    #[must_use]
    pub fn single(order: &'a Builder) -> Self {
        Self {
            purchase_orders: vec![order],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::line_item;
    use rust_decimal_macros::dec;
    use serde_json::Value;

    #[test]
    fn purchase_order_request_serializes_as_spec() {
        let contact_id = Uuid::nil();
        let line_item_builder = line_item::Builder::new(
            Some("Sample item".to_string()),
            Some(dec!(1)),
            Some(dec!(10)),
        );
        let builder = Builder::new(ContactIdentifier::ID(contact_id), vec![line_item_builder]);
        let request = PurchaseOrdersRequest::single(&builder);
        let json = serde_json::to_value(&request).expect("serialization should succeed");

        let orders = json
            .get("PurchaseOrders")
            .and_then(Value::as_array)
            .expect("PurchaseOrders array expected");
        assert_eq!(orders.len(), 1);
        let order = orders[0].as_object().expect("order object expected");
        let contact = order
            .get("Contact")
            .and_then(Value::as_object)
            .expect("contact object expected");
        let contact_id_str = contact_id.to_string();
        assert_eq!(
            contact.get("ContactID").and_then(Value::as_str),
            Some(contact_id_str.as_str())
        );
        assert!(!order.contains_key("Reference"));
    }

    #[test]
    fn builder_default_omits_contact_for_partial_updates() {
        // This tests that Builder::default() can be used for partial updates
        // without accidentally including a contact field
        let mut builder = Builder::default();
        builder.reference = Some("Updated reference".to_string());
        builder.purchase_order_id = Some(Uuid::nil());

        let request = PurchaseOrdersRequest::single(&builder);
        let json = serde_json::to_value(&request).expect("serialization should succeed");

        let orders = json
            .get("PurchaseOrders")
            .and_then(Value::as_array)
            .expect("PurchaseOrders array expected");
        let order = orders[0].as_object().expect("order object expected");

        // Contact should be omitted when None
        assert!(
            !order.contains_key("Contact"),
            "Contact should not be serialized when None"
        );
        // But Reference and PurchaseOrderID should be present
        assert_eq!(
            order.get("Reference").and_then(Value::as_str),
            Some("Updated reference")
        );
        assert!(order.contains_key("PurchaseOrderID"));
    }
}
