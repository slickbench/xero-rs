use time::OffsetDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::Result, 
    Client,
    utils::date_format::xero_datetime_format,
};

pub const ENDPOINT: &str = "https://api.xero.com/connections";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub id: Uuid,
    pub auth_event_id: Uuid,
    pub tenant_id: Uuid,
    pub tenant_type: String,
    pub tenant_name: String,
    #[serde(with = "xero_datetime_format")]
    pub created_date_utc: OffsetDateTime,
    #[serde(with = "xero_datetime_format")]
    pub updated_date_utc: OffsetDateTime,
}

/// Retrieve a list of authorized connections (tennants).
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<Connection>> {
    client.get(ENDPOINT, Vec::<String>::default()).await
}
