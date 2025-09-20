#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::{Client, oauth::KeyPair};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Create a client with client credentials and required scopes
    let client = Client::from_client_credentials(
        KeyPair::from_env(),
        Some(xero_rs::scope::Scope::common_accounting_read()),
    )
    .await?;

    // List available connections (tenants)
    let connections = xero_rs::entities::connection::list(&client).await?;
    info!("found client connections: {:#?}", connections);

    // Select the first tenant and set it on the client
    let tenant_id = connections.first().expect("No connections found").tenant_id;
    client.set_tenant(Some(tenant_id)).await;

    // List invoices
    let invoices = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await?;
    info!("found {} invoices", invoices.len());

    Ok(())
}
