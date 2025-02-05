#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::KeyPair;
use xero_rs::invoice::ListParameters;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,xero_rs=trace")
        .init();
    let client = xero_rs::Client::from_client_credentials(KeyPair::from_env(), None).await?;

    let connections = xero_rs::connection::list(&client).await?;
    info!("found client connections: {:#?}", connections);

    let invoices = xero_rs::invoice::list(&client, ListParameters::default()).await?;
    info!("found invoices: {:#?}", invoices);

    Ok(())
}
