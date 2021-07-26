#[macro_use]
extern crate tracing;

use anyhow::Result;
use oauth2::{ClientId, ClientSecret};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("debug,xero_rs=trace")
        .init();
    let client = xero_rs::Client::new_with_client_credentials(
        ClientId::new(std::env::var("XERO_CLIENT_ID")?),
        Some(ClientSecret::new(std::env::var("XERO_CLIENT_SECRET")?)),
        None,
    )
    .await?;

    let connections = client.get_connections().await?;
    info!("received client connections: {:#?}", connections);

    Ok(())
}
