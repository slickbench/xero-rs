use uuid::Uuid;
use chrono::NaiveDateTime;
use serde::{Serialize, Deserialize};

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
