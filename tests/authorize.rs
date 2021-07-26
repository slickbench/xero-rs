#[macro_use]
extern crate tracing;

use std::env;

use anyhow::Result;
use oauth2::{ClientId, ClientSecret};

#[tokio::test]
async fn authorize_client() -> Result<()> {
    let client = xero_rs::Client::new_with_client_credentials(
        ClientId::new(env::var("XERO_CLIENT_ID")?),
        Some(ClientSecret::new(env::var("XERO_CLIENT_SECRET")?)),
        None,
    )
    .await?;

    let connections = client.get_connections().await?;
    info!("received client connections: {:?}", connections);
    Ok(())
}
