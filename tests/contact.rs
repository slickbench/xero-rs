#[macro_use]
extern crate tracing;

mod test_utils;

use anyhow::Result;
use std::env;
use xero_rs::KeyPair;

#[tokio::test]
async fn list_contacts() -> Result<()> {
    test_utils::do_setup();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");

    // Create client with credentials (use tenant discovery)
    let client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        None, // Use app default scopes
    )
    .await?;

    // Discover and set tenant
    let connections = xero_rs::connection::list(&client).await?;
    let tenant = connections
        .first()
        .expect("No tenants connected to this app");
    info!(
        "Using tenant: {} ({})",
        tenant.tenant_name, tenant.tenant_id
    );
    client.set_tenant(Some(tenant.tenant_id)).await;

    // List contacts
    let contacts = client.contacts().list().await?;
    info!("Found {} contacts", contacts.len());
    Ok(())
}

#[tokio::test]
async fn get_contact() -> Result<()> {
    test_utils::do_setup();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");

    // Create client with credentials (use tenant discovery)
    let client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        None,
    )
    .await?;

    // Discover and set tenant
    let connections = xero_rs::connection::list(&client).await?;
    let tenant = connections
        .first()
        .expect("No tenants connected to this app");
    client.set_tenant(Some(tenant.tenant_id)).await;

    // First list contacts to get an ID
    let contacts = client.contacts().list().await?;

    if contacts.is_empty() {
        info!("No contacts found, skipping get_contact test");
        return Ok(());
    }

    // Get the first contact by ID
    let contact_id = contacts[0].contact_id;
    let contact = client.contacts().get(contact_id).await?;

    info!("Got contact: {} ({})", contact.name, contact.contact_id);
    assert_eq!(contact.contact_id, contact_id);

    Ok(())
}
