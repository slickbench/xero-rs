#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::invoice::ListParameters;
use xero_rs::KeyPair;

#[tokio::test]
async fn get_invoices() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trace")
        .with_test_writer()
        .init();
    let client = xero_rs::Client::from_client_credentials(KeyPair::from_env(), None).await?;

    let invoices = xero_rs::invoice::list(&client, ListParameters::default()).await?;
    debug!("found {:?} invoices", invoices.len());

    let invoice_from_list = invoices.first().unwrap();
    let invoice = xero_rs::invoice::get(&client, invoice_from_list.invoice_id).await?;
    assert_eq!(invoice_from_list.invoice_id, invoice.invoice_id);

    Ok(())
}
