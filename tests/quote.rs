#[macro_use]
extern crate tracing;

use std::env;
use anyhow::Result;
use uuid::Uuid;
use xero_rs::{KeyPair, XeroScope};

mod test_utils;

#[tokio::test]
async fn get_quotes() -> Result<()> {
    test_utils::do_setup();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");
    let tenant_id = Uuid::parse_str(&env::var("XERO_TENANT_ID").expect("XERO_TENANT_ID must be set"))
        .expect("Invalid XERO_TENANT_ID format");

    // Create client with credentials and scopes
    let mut client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        Some(vec![
            XeroScope::accounting_transactions_read(),
        ]),
    ).await?;

    // Set the tenant ID
    client.set_tenant(Some(tenant_id));

    let quotes = xero_rs::quote::list(&client).await?;
    debug!("found {} quotes", quotes.len());

    if let Some(quote_from_list) = quotes.first() {
        let quote = xero_rs::quote::get(&client, quote_from_list.quote_id).await?;
        assert_eq!(quote_from_list.quote_id, quote.quote_id);
    } else {
        debug!("No quotes found in the account - skipping individual quote fetch test");
    }

    Ok(())
}
