//! Example demonstrating automatic token refresh functionality
//!
//! This example shows how to enable automatic token refresh so that
//! the client automatically handles token expiry without manual intervention.

#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::{Client, oauth::KeyPair};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load credentials from environment variables
    let key_pair = KeyPair::from_env();

    // Create a client with auto-refresh enabled
    // Method 1: Clone the key_pair for use in both places
    let client = Client::from_client_credentials(
        key_pair.clone(),
        Some(xero_rs::scope::Scope::common_accounting_read()),
    )
    .await?
    .with_auto_refresh(key_pair); // Enable auto-refresh

    info!("Created client with auto-refresh enabled");

    // List available connections (tenants)
    let connections = xero_rs::entities::connection::list(&client).await?;
    info!("Found {} connections", connections.len());

    // Select the first tenant and set it on the client
    if let Some(connection) = connections.first() {
        let tenant_id = connection.tenant_id;
        client.set_tenant(Some(tenant_id)).await;
        info!("Set tenant ID: {}", tenant_id);
    } else {
        error!("No connections found");
        return Ok(());
    }

    // Make API requests - the client will automatically refresh the token if it expires
    info!("Making API requests...");

    // First request
    let invoices = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await?;
    info!("Found {} invoices", invoices.len());

    // If you make many requests over a long period, the token might expire.
    // With auto-refresh enabled, the client will automatically refresh the token
    // and retry the request transparently.

    // Simulate a long-running process that might exceed token lifetime
    info!("Starting long-running process...");
    for i in 0..3 {
        // Sleep for a while (in a real app, this might be processing time)
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // Make another request - will auto-refresh if token expired
        let contacts = client.contacts().list().await?;
        info!("Iteration {}: Found {} contacts", i + 1, contacts.len());
    }

    info!("All requests completed successfully with auto-refresh handling token expiry");

    Ok(())
}

// Alternative approach: Creating a client without auto-refresh and handling manually
#[allow(dead_code)]
async fn example_without_auto_refresh() -> Result<()> {
    let key_pair = KeyPair::from_env();

    // Create client without auto-refresh
    let mut client = Client::from_client_credentials(
        key_pair.clone(),
        Some(xero_rs::scope::Scope::common_accounting_read()),
    )
    .await?
    .without_auto_refresh(); // Explicitly disable auto-refresh

    // Now you need to handle token refresh manually if needed
    match client.invoices().list_all().await {
        Ok(invoices) => {
            info!("Found {} invoices", invoices.len());
        }
        Err(xero_rs::error::Error::API(api_err))
            if matches!(
                api_err.error,
                xero_rs::error::ErrorType::UnauthorisedException
            ) =>
        {
            // Token expired, manually refresh
            warn!("Token expired, manually refreshing...");
            client.refresh_access_token(key_pair).await?;

            // Retry the request
            let invoices = client.invoices().list_all().await?;
            info!("Found {} invoices after manual refresh", invoices.len());
        }
        Err(e) => return Err(e.into()),
    }

    Ok(())
}
