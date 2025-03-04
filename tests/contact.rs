#[macro_use]
extern crate tracing;

mod test_utils;

use anyhow::Result;
use std::env;
use uuid::Uuid;
use xero_rs::{KeyPair, Scope};

#[tokio::test]
async fn list_contacts() -> Result<()> {
    test_utils::do_setup();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");
    let tenant_id =
        Uuid::parse_str(&env::var("XERO_TENANT_ID").expect("XERO_TENANT_ID must be set"))
            .expect("Invalid XERO_TENANT_ID format");

    // Create client with credentials and scopes
    let mut client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        Some(vec![Scope::accounting_contacts_read()]),
    )
    .await?;

    // Set the tenant ID
    client.set_tenant(Some(tenant_id));

    let contacts = xero_rs::contact::list(&client).await?;
    info!("received contacts: {:?}", contacts);
    Ok(())
}
