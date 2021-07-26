#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::KeyPair;

#[tokio::test]
async fn get_quotes() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trace")
        .with_test_writer()
        .init();
    let client = xero_rs::Client::from_client_credentials(KeyPair::from_env(), None).await?;

    let quotes = xero_rs::quote::list(&client).await?;
    debug!("found {:?} quotes", quotes.len());

    let quote_from_list = quotes.first().unwrap();
    let quote = xero_rs::quote::get(&client, quote_from_list.quote_id).await?;
    assert_eq!(quote_from_list.quote_id, quote.quote_id);

    Ok(())
}
