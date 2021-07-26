use serde::Deserialize;
use uuid::Uuid;

use self::{contact::Contact, invoice::Invoice, purchase_order::PurchaseOrder, quote::Quote};

pub mod connection;
pub mod contact;
pub mod invoice;
pub mod line_item;
pub mod purchase_order;
pub mod quote;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Data {
    PurchaseOrders(Vec<PurchaseOrder>),
    Invoices(Vec<Invoice>),
    Contacts(Vec<Contact>),
    Quotes(Vec<Quote>),
}

impl Data {
    #[must_use]
    pub fn get_purchase_orders(self) -> Option<Vec<PurchaseOrder>> {
        if let Self::PurchaseOrders(purchase_orders) = self {
            Some(purchase_orders)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_invoices(self) -> Option<Vec<Invoice>> {
        if let Self::Invoices(invoices) = self {
            Some(invoices)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_contacts(self) -> Option<Vec<Contact>> {
        if let Self::Contacts(contacts) = self {
            Some(contacts)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get_quotes(self) -> Option<Vec<Quote>> {
        if let Self::Quotes(quotes) = self {
            Some(quotes)
        } else {
            None
        }
    }
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MutationStatus {
    OK,
}

/// Represents the structure returned by the Xero API when inserting, updating, or deleting
/// objects.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MutationResponse {
    pub id: Uuid,
    pub status: MutationStatus,
    pub provider_name: String,
    #[serde(rename = "DateTimeUTC")]
    pub date_time_utc: String,

    #[serde(flatten)]
    pub data: Data,
}
