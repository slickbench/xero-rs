use std::{convert::TryFrom, fmt};
use url::Url;
use uuid::Uuid;

use crate::error::{Error, Result};

pub const BASE_URL: &str = "https://api.xero.com/api.xro/2.0/";

/// A typed representation of Xero API endpoints.
///
/// This enum represents all the possible endpoints for the Xero API,
/// providing a type-safe way to construct API URLs.
#[derive(Debug, Clone)]
pub enum XeroEndpoint {
    // Accounting endpoints
    Accounts,
    Account(Uuid),
    Contacts,
    Contact(Uuid),
    Invoices,
    Invoice(Uuid),
    Items,
    Item(Uuid),
    PurchaseOrders,
    PurchaseOrder(Uuid),
    Quotes,
    Quote(Uuid),

    // Payroll endpoints
    Timesheets,
    Timesheet(Uuid),

    // Custom endpoint with path components
    Custom(Vec<String>),
}

impl XeroEndpoint {
    /// Converts the endpoint to a URL string.
    pub fn to_url(&self) -> Result<Url> {
        let base = Url::parse(BASE_URL).map_err(|_| Error::InvalidEndpoint)?;

        let path = match self {
            Self::Accounts => "Accounts",
            Self::Account(id) => {
                return base
                    .join(&format!("Accounts/{id}"))
                    .map_err(|_| Error::InvalidEndpoint);
            }
            Self::Contacts => "Contacts",
            Self::Contact(id) => {
                return base
                    .join(&format!("Contacts/{id}"))
                    .map_err(|_| Error::InvalidEndpoint);
            }
            Self::Invoices => "Invoices",
            Self::Invoice(id) => {
                return base
                    .join(&format!("Invoices/{id}"))
                    .map_err(|_| Error::InvalidEndpoint);
            }
            Self::Items => "Items",
            Self::Item(id) => {
                return base
                    .join(&format!("Items/{id}"))
                    .map_err(|_| Error::InvalidEndpoint);
            }
            Self::PurchaseOrders => "PurchaseOrders",
            Self::PurchaseOrder(id) => {
                return base
                    .join(&format!("PurchaseOrders/{id}"))
                    .map_err(|_| Error::InvalidEndpoint);
            }
            Self::Quotes => "Quotes",
            Self::Quote(id) => {
                return base
                    .join(&format!("Quotes/{id}"))
                    .map_err(|_| Error::InvalidEndpoint);
            }
            Self::Timesheets => "Timesheets",
            Self::Timesheet(id) => {
                return base
                    .join(&format!("Timesheets/{id}"))
                    .map_err(|_| Error::InvalidEndpoint);
            }
            Self::Custom(components) => {
                return {
                    let path = components.join("/");
                    base.join(&path).map_err(|_| Error::InvalidEndpoint)
                };
            }
        };

        base.join(path).map_err(|_| Error::InvalidEndpoint)
    }

    /// Creates a custom endpoint from a full URL string
    #[must_use]
    pub fn from_string(url: String) -> Self {
        Self::Custom(vec![url])
    }
}

impl fmt::Display for XeroEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_url() {
            Ok(url) => write!(f, "{url}"),
            Err(_) => write!(f, "Invalid endpoint"),
        }
    }
}

// Allow conversion from XeroEndpoint to a string URL
impl From<XeroEndpoint> for String {
    fn from(endpoint: XeroEndpoint) -> Self {
        endpoint.to_string()
    }
}

// Allow conversion from XeroEndpoint to a Url
impl TryFrom<XeroEndpoint> for Url {
    type Error = Error;

    fn try_from(endpoint: XeroEndpoint) -> Result<Self> {
        endpoint.to_url()
    }
}
