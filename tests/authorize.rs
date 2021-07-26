#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::KeyPair;

#[tokio::test]
async fn authorize_client() -> Result<()> {
    let client = xero_rs::Client::from_client_credentials(KeyPair::from_env(), None).await?;

    let connections = xero_rs::connection::list(&client).await?;
    info!("received client connections: {:?}", connections);
    Ok(())
}
