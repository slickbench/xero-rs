use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::Result, Client};

pub const ENDPOINT: &str = "https://api.xero.com/connections";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    id: Uuid,
    auth_event_id: Uuid,
    tenant_id: Uuid,
    tenant_type: String,
    tenant_name: String,
    created_date_utc: NaiveDateTime,
    updated_date_utc: NaiveDateTime,
}

/// Retrieve a list of authorized connections (tennants).
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<Connection>> {
    client.get(ENDPOINT, Vec::<String>::default()).await
}
