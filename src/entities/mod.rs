use serde::Deserialize;
use uuid::Uuid;

use self::{
    contact::Contact,
    invoice::Invoice,
    purchase_order::PurchaseOrder,
    quote::Quote,
    timesheet::Timesheet,
};

pub mod connection;
pub mod contact;
pub mod invoice;
pub mod line_item;
pub mod purchase_order;
pub mod quote;
pub mod timesheet;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Data {
    PurchaseOrders(Vec<PurchaseOrder>),
    Invoices(Vec<Invoice>),
    Contacts(Vec<Contact>),
    Quotes(Vec<Quote>),
    Timesheets(Vec<Timesheet>),
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

    #[must_use]
    pub fn get_timesheets(self) -> Option<Vec<Timesheet>> {
        if let Self::Timesheets(timesheets) = self {
            Some(timesheets)
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

/// Generic trait for standard CRUD operations on Xero entities
pub trait EntityEndpoint<T, ListParams = ()> {
    /// The API endpoint URL for this entity
    fn endpoint() -> &'static str;
    
    /// Get entity by ID
    async fn get(client: &crate::Client, id: uuid::Uuid) -> crate::error::Result<T>;
    
    /// List entities with optional parameters
    async fn list(client: &crate::Client, params: ListParams) -> crate::error::Result<Vec<T>>;
}

/// Generic implementation for entity CRUD operations
pub mod endpoint_utils {
    use serde::de::DeserializeOwned;
    use url::Url;
    use std::str::FromStr;
    use uuid::Uuid;
    
    use crate::{Client, error::{Error, Result}};
    
    /// Generic function to get a single entity by ID
    pub async fn get<T, R>(
        client: &Client, 
        endpoint: &str, 
        id: Uuid, 
        entity_name: &str
    ) -> Result<T> 
    where
        R: DeserializeOwned + Into<Vec<T>>,
    {
        let endpoint = Url::from_str(endpoint)
            .and_then(|endpoint| endpoint.join(&id.to_string()))
            .map_err(|_| Error::InvalidEndpoint)?;
        let endpoint_str = endpoint.to_string();
        let response: R = client.get(endpoint, Vec::<String>::default()).await?;
        response.into().into_iter().next().ok_or(Error::NotFound {
            entity: entity_name.to_string(),
            url: endpoint_str,
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some(format!("{} with ID {} not found", entity_name, id)),
        })
    }
    
    /// Generic function to list entities with optional parameters
    pub async fn list<T, R, P>(
        client: &Client,
        endpoint: &str,
        params: P,
    ) -> Result<Vec<T>>
    where
        R: DeserializeOwned + Into<Vec<T>>,
        P: serde::Serialize + std::fmt::Debug,
    {
        let response: R = client.get(endpoint, params).await?;
        Ok(response.into())
    }
}

/// Trait for entity builders
pub trait EntityBuilder<T> {
    /// Build and create the entity via the API
    async fn create(self, client: &crate::Client) -> crate::error::Result<T>;
}

/// Helper functions for entity creation
pub mod builder_utils {
    use serde::{de::DeserializeOwned, Serialize};
    use uuid::Uuid;
    
    use crate::{Client, error::{Error, Result}};
    
    /// Generic function to create a new entity
    pub async fn create<T, R, B>(
        client: &Client,
        endpoint: &str,
        builder: &B,
    ) -> Result<T>
    where
        T: DeserializeOwned,
        R: DeserializeOwned + Into<Option<T>>,
        B: Serialize + std::fmt::Debug,
    {
        let response: R = client.post(endpoint, builder).await?;
        response.into().ok_or_else(|| Error::NotFound {
            entity: std::any::type_name::<T>().to_string(),
            url: endpoint.to_string(),
            status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            response_body: Some("Failed to create entity".to_string()),
        })
    }
}

/// Trait for standardizing API response handling
pub trait ApiResponse<T> {
    /// Convert the API response to the target type
    fn into_result(self) -> crate::error::Result<T>;
}

/// Generic implementation for a list response
#[derive(Debug, serde::Deserialize)]
pub struct ListResponse<T, K: AsRef<str>> {
    #[serde(rename = "PascalCase")]
    items: Vec<T>,
    #[serde(skip)]
    _key: std::marker::PhantomData<K>,
}

impl<T, K: AsRef<str>> ApiResponse<Vec<T>> for ListResponse<T, K> {
    fn into_result(self) -> crate::error::Result<Vec<T>> {
        Ok(self.items)
    }
}

/// Generic implementation for a single entity response
#[derive(Debug, serde::Deserialize)]
pub struct SingleResponse<T, K: AsRef<str>> {
    #[serde(rename = "PascalCase")]
    item: T,
    #[serde(skip)]
    _key: std::marker::PhantomData<K>,
}

impl<T, K: AsRef<str>> ApiResponse<T> for SingleResponse<T, K> {
    fn into_result(self) -> crate::error::Result<T> {
        Ok(self.item)
    }
}
