#[macro_use]
extern crate tracing;

mod test_utils;

use anyhow::Result;
use std::env;
use xero_rs::KeyPair;

#[tokio::test]
async fn test_method_based_api() -> Result<()> {
    test_utils::do_setup();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");

    // Create client with credentials and scopes
    // Use None to get default scopes configured in the Xero app
    let client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        None,
    )
    .await?;

    // Discover tenants and use the first one
    let connections = xero_rs::connection::list(&client).await?;
    info!("Found {} connected tenants", connections.len());

    let tenant = connections
        .first()
        .expect("No tenants connected to this app");
    info!(
        "Using tenant: {} ({})",
        tenant.tenant_name, tenant.tenant_id
    );
    client.set_tenant(Some(tenant.tenant_id)).await;

    // Use the new method-based API
    info!("=== Using method-based API ===");

    // List contacts
    let contacts = client.contacts().list().await?;
    info!("Found {} contacts", contacts.len());

    if !contacts.is_empty() {
        // Get a specific contact
        let contact = client.contacts().get(contacts[0].contact_id).await?;
        info!("Got contact: {}", contact.name);
    }

    // List invoices
    let invoices = client.invoices().list_all().await?;
    info!("Found {} invoices", invoices.len());

    if !invoices.is_empty() {
        // Get a specific invoice
        let invoice = client.invoices().get(invoices[0].invoice_id).await?;
        info!("Got invoice #{}", invoice.invoice_id);
    }

    // List purchase orders
    let purchase_orders = client.purchase_orders().list().await?;
    info!("Found {} purchase orders", purchase_orders.len());

    if !purchase_orders.is_empty() {
        // Get a specific purchase order
        let purchase_order = client
            .purchase_orders()
            .get(purchase_orders[0].purchase_order_id)
            .await?;
        info!("Got purchase order #{}", purchase_order.purchase_order_id);
    }

    // List quotes
    let quotes = client
        .quotes()
        .list(xero_rs::quote::ListParameters::default())
        .await?;
    info!("Found {} quotes", quotes.len());

    if !quotes.is_empty() {
        // Get a specific quote
        let quote = client.quotes().get(quotes[0].quote_id).await?;
        info!("Got quote #{}", quote.quote_id);
    }

    Ok(())
}
