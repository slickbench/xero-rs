#[macro_use]
extern crate tracing;

mod test_utils;

use std::env;
use anyhow::Result;
use uuid::Uuid;
use xero_rs::{invoice::ListParameters, KeyPair, XeroScope};

#[tokio::test]
async fn get_invoices() -> Result<()> {
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

    let invoices = xero_rs::invoice::list(&client, ListParameters::default()).await?;
    debug!("found {:?} invoices", invoices.len());

    let invoice_from_list = invoices.first().unwrap();
    let invoice = xero_rs::invoice::get(&client, invoice_from_list.invoice_id).await?;
    assert_eq!(invoice_from_list.invoice_id, invoice.invoice_id);

    Ok(())
}
