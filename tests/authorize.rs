#[macro_use]
extern crate tracing;

mod test_utils;

use anyhow::Result;
use std::env;
use uuid::Uuid;
use xero_rs::KeyPair;

#[tokio::test]
async fn authorize_client() -> Result<()> {
    test_utils::do_setup();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");
    let tenant_id =
        Uuid::parse_str(&env::var("XERO_TENANT_ID").expect("XERO_TENANT_ID must be set"))
            .expect("Invalid XERO_TENANT_ID format");

    // Create client with credentials and scopes
    let client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        xero_rs::Scope::accounting_settings_read(),
    )
    .await?;

    // Set the tenant ID
    client.set_tenant(Some(tenant_id)).await;

    let connections = xero_rs::connection::list(&client).await?;
    info!("received client connections: {:?}", connections);
    Ok(())
}
