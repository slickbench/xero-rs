use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    Client,
    endpoints::XeroEndpoint,
    entities::{EntityEndpoint, MutationResponse, endpoint_utils},
    error::{Error, Result},
    utils::date_format::xero_datetime_format,
};

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/Items/";

/// Details for purchasing an item
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PurchaseDetails {
    /// Unit price for purchasing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_price: Option<Decimal>,

    /// Account code for cost of goods sold, only applicable for non-tracked inventory items
    #[serde(rename = "AccountCode", skip_serializing_if = "Option::is_none")]
    pub account_code: Option<String>,

    /// Account code for cost of goods sold, only applicable for tracked inventory items
    #[serde(rename = "COGSAccountCode", skip_serializing_if = "Option::is_none")]
    pub cogs_account_code: Option<String>,

    /// Tax type for purchasing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,
}

/// Details for selling an item
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SalesDetails {
    /// Unit price for selling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_price: Option<Decimal>,

    /// Account code for sales
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_code: Option<String>,

    /// Tax type for sales
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,
}

/// Represents an inventory item or service
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Item {
    /// Unique identifier for the item
    #[serde(rename = "ItemID")]
    pub item_id: Uuid,

    /// User-defined item code (must be unique)
    pub code: String,

    /// Name of the item (max length 50)
    pub name: String,

    /// Description of the item (max length 4000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Purchase description (max length 4000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_description: Option<String>,

    /// Purchase details for the item
    #[serde(default)]
    pub purchase_details: PurchaseDetails,

    /// Sales details for the item
    #[serde(default)]
    pub sales_details: SalesDetails,

    /// True if item is tracked as inventory
    #[serde(default)]
    pub is_tracked_as_inventory: bool,

    /// The inventory asset account code for tracked items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory_asset_account_code: Option<String>,

    /// The total cost pool for tracked items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_pool: Option<Decimal>,

    /// The quantity on hand for tracked items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity_on_hand: Option<Decimal>,

    /// True if item is sold
    #[serde(default)]
    pub is_sold: bool,

    /// True if item is purchased
    #[serde(default)]
    pub is_purchased: bool,

    /// Last modified date
    #[serde(rename = "UpdatedDateUTC", with = "xero_datetime_format")]
    pub updated_date_utc: OffsetDateTime,

    /// Validation errors from the API
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_errors: Vec<ValidationError>,
}

/// Validation error returned by the API
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ValidationError {
    pub message: String,
}

/// Response wrapper for listing items
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ListResponse {
    pub items: Vec<Item>,
    pub status: String,
    pub id: Uuid,
    pub provider_name: String,
    #[serde(rename = "DateTimeUTC")]
    pub date_time_utc: Option<String>,
}

impl From<ListResponse> for Vec<Item> {
    fn from(response: ListResponse) -> Self {
        response.items
    }
}

/// Parameters for listing items
#[derive(Debug, Serialize, Default)]
pub struct ListParameters {
    /// Filter by any element
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,

    /// Order by any element
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,

    /// Number of decimal places for unit amounts
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unitdp: Option<u8>,
}

impl ListParameters {
    /// Create a new builder for ListParameters
    #[must_use]
    pub fn builder() -> Self {
        Self::default()
    }

    /// Set the where filter
    #[must_use]
    pub fn with_where(mut self, filter: impl Into<String>) -> Self {
        self.r#where = Some(filter.into());
        self
    }

    /// Set the order clause
    #[must_use]
    pub fn with_order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    /// Set the unit decimal places
    #[must_use]
    pub fn with_unitdp(mut self, unitdp: u8) -> Self {
        self.unitdp = Some(unitdp);
        self
    }
}

/// Builder for creating or updating items
#[derive(Debug, Serialize, Clone, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    /// User-defined item code (must be unique)
    pub code: String,

    /// Name of the item (max length 50)
    pub name: String,

    /// Description of the item (max length 4000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Purchase description (max length 4000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_description: Option<String>,

    /// Purchase details for the item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_details: Option<PurchaseDetails>,

    /// Sales details for the item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sales_details: Option<SalesDetails>,

    /// True if item is tracked as inventory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_tracked_as_inventory: Option<bool>,

    /// The inventory asset account code for tracked items
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inventory_asset_account_code: Option<String>,

    /// True if item is sold
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_sold: Option<bool>,

    /// True if item is purchased
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_purchased: Option<bool>,

    /// Item ID for updates
    #[serde(rename = "ItemID", skip_serializing_if = "Option::is_none")]
    pub item_id: Option<Uuid>,
}

impl Builder {
    /// Create a new item builder
    #[must_use]
    pub fn new(code: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the purchase description
    #[must_use]
    pub fn with_purchase_description(mut self, description: impl Into<String>) -> Self {
        self.purchase_description = Some(description.into());
        self
    }

    /// Set the purchase details
    #[must_use]
    pub fn with_purchase_details(mut self, details: PurchaseDetails) -> Self {
        self.purchase_details = Some(details);
        self
    }

    /// Set the sales details
    #[must_use]
    pub fn with_sales_details(mut self, details: SalesDetails) -> Self {
        self.sales_details = Some(details);
        self
    }

    /// Set whether the item is tracked as inventory
    #[must_use]
    pub fn with_is_tracked_as_inventory(mut self, tracked: bool) -> Self {
        self.is_tracked_as_inventory = Some(tracked);
        self
    }

    /// Set the inventory asset account code
    #[must_use]
    pub fn with_inventory_asset_account_code(mut self, code: impl Into<String>) -> Self {
        self.inventory_asset_account_code = Some(code.into());
        self
    }

    /// Set whether the item is sold
    #[must_use]
    pub fn with_is_sold(mut self, sold: bool) -> Self {
        self.is_sold = Some(sold);
        self
    }

    /// Set whether the item is purchased
    #[must_use]
    pub fn with_is_purchased(mut self, purchased: bool) -> Self {
        self.is_purchased = Some(purchased);
        self
    }

    /// Set the item ID (for updates)
    #[must_use]
    pub fn with_item_id(mut self, id: Uuid) -> Self {
        self.item_id = Some(id);
        self
    }
}

/// Request wrapper for items
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ItemWrapper<'a> {
    pub items: Vec<&'a Builder>,
}

/// History record for an item
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecord {
    /// The details of the history record
    pub details: String,

    /// The date and time of the history record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_utc: Option<String>,

    /// The user who created the history record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// The changes made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<String>,
}

/// Wrapper for history records response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecords {
    pub history_records: Vec<HistoryRecord>,
}

/// Wrapper for posting history records
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecordsRequest {
    pub history_records: Vec<HistoryRecord>,
}

impl EntityEndpoint<Item, ListParameters> for Item {
    fn endpoint() -> &'static str {
        ENDPOINT
    }

    async fn get(client: &mut Client, id: Uuid) -> Result<Item> {
        endpoint_utils::get::<Item, ListResponse>(client, ENDPOINT, id, "Item").await
    }

    async fn list(client: &mut Client, params: ListParameters) -> Result<Vec<Item>> {
        endpoint_utils::list::<Item, ListResponse, ListParameters>(client, ENDPOINT, &params).await
    }
}

// Add extension methods to Item
impl Item {
    /// Get a single item by code
    pub async fn get_by_code(client: &mut Client, code: &str) -> Result<Item> {
        use std::str::FromStr;
        use url::Url;

        let endpoint = Url::from_str(ENDPOINT)
            .and_then(|endpoint| endpoint.join(code))
            .map_err(|_| Error::InvalidEndpoint)?;
        let endpoint_str = endpoint.to_string();
        let empty_vec: Vec<String> = Vec::new();
        let response: ListResponse = client.get(endpoint, &empty_vec).await?;
        let items = Vec::from(response);
        items.into_iter().next().ok_or(Error::NotFound {
            entity: "Item".to_string(),
            url: endpoint_str,
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some(format!("Item with code {} not found", code)),
        })
    }
}

/// List items with optional parameters
pub async fn list(client: &mut Client, params: ListParameters) -> Result<Vec<Item>> {
    Item::list(client, params).await
}

/// List all items without any filtering
pub async fn list_all(client: &mut Client) -> Result<Vec<Item>> {
    Item::list(client, ListParameters::default()).await
}

/// Get a single item by ID
pub async fn get(client: &mut Client, item_id: Uuid) -> Result<Item> {
    Item::get(client, item_id).await
}

/// Get a single item by code
pub async fn get_by_code(client: &mut Client, code: &str) -> Result<Item> {
    Item::get_by_code(client, code).await
}

/// Create one or more items
pub async fn create(client: &mut Client, items: &[Builder]) -> Result<Vec<Item>> {
    let wrapper = ItemWrapper {
        items: items.iter().collect(),
    };

    let response: MutationResponse = client
        .put_endpoint(XeroEndpoint::Custom(vec!["Items".to_string()]), &wrapper)
        .await?;

    response.data.get_items().ok_or(Error::NotFound {
        entity: "Item".to_string(),
        url: ENDPOINT.to_string(),
        status_code: reqwest::StatusCode::NOT_FOUND,
        response_body: Some("No items returned in response".to_string()),
    })
}

/// Create a single item
pub async fn create_single(client: &mut Client, item: &Builder) -> Result<Item> {
    let items = create(client, &[item.clone()]).await?;
    items.into_iter().next().ok_or(Error::NotFound {
        entity: "Item".to_string(),
        url: ENDPOINT.to_string(),
        status_code: reqwest::StatusCode::NOT_FOUND,
        response_body: Some("No item returned in response".to_string()),
    })
}

/// Update or create one or more items
pub async fn update_or_create(client: &mut Client, items: &[Builder]) -> Result<Vec<Item>> {
    let wrapper = ItemWrapper {
        items: items.iter().collect(),
    };

    let response: MutationResponse = client
        .post_endpoint(XeroEndpoint::Custom(vec!["Items".to_string()]), &wrapper)
        .await?;

    response.data.get_items().ok_or(Error::NotFound {
        entity: "Item".to_string(),
        url: ENDPOINT.to_string(),
        status_code: reqwest::StatusCode::NOT_FOUND,
        response_body: Some("No items returned in response".to_string()),
    })
}

/// Update a specific item
pub async fn update(client: &mut Client, item_id: Uuid, item: &Builder) -> Result<Item> {
    let mut item_with_id = item.clone();
    item_with_id.item_id = Some(item_id);

    let wrapper = ItemWrapper {
        items: vec![&item_with_id],
    };

    let endpoint = XeroEndpoint::Custom(vec![format!("Items/{}", item_id)]);
    let response: MutationResponse = client.post_endpoint(endpoint, &wrapper).await?;

    response
        .data
        .get_items()
        .and_then(|items| items.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Item".to_string(),
            url: format!("{}{}", ENDPOINT, item_id),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some("No item returned in response".to_string()),
        })
}

/// Delete a specific item
pub async fn delete(client: &mut Client, item_id: Uuid) -> Result<()> {
    let endpoint = XeroEndpoint::Custom(vec![format!("Items/{}", item_id)]);
    client.delete_endpoint(endpoint).await
}

/// Get the history for an item
pub async fn get_history(client: &mut Client, item_id: Uuid) -> Result<Vec<HistoryRecord>> {
    let endpoint = XeroEndpoint::Custom(vec![format!("Items/{}/History", item_id)]);
    let response: HistoryRecords = client.get_endpoint(endpoint, &()).await?;
    Ok(response.history_records)
}

/// Create a history record for an item
pub async fn create_history(
    client: &mut Client,
    item_id: Uuid,
    details: &str,
) -> Result<Vec<HistoryRecord>> {
    let history_record = HistoryRecord {
        details: details.to_string(),
        date_utc: None,
        user: None,
        changes: None,
    };

    let request = HistoryRecordsRequest {
        history_records: vec![history_record],
    };

    let endpoint = XeroEndpoint::Custom(vec![format!("Items/{}/History", item_id)]);
    let response: HistoryRecords = client.put_endpoint(endpoint, &request).await?;
    Ok(response.history_records)
}
