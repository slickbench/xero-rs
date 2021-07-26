#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::KeyPair;

#[tokio::test]
async fn list_contacts() -> Result<()> {
    tracing_subscriber::fmt().with_test_writer().init();
    let client = xero_rs::Client::from_client_credentials(KeyPair::from_env(), None).await?;

    let contacts = xero_rs::contact::list(&client).await?;
    info!("received contacts: {:?}", contacts);
    Ok(())
}
