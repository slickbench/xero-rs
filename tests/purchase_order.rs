#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::KeyPair;

#[tokio::test]
async fn get_purchase_orders() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trace")
        .with_test_writer()
        .init();
    let client = xero_rs::Client::from_client_credentials(KeyPair::from_env(), None).await?;

    let purchase_orders = xero_rs::purchase_order::list(&client).await?;
    debug!("found {:?} purchase_orders", purchase_orders.len());

    let purchase_order_from_list = purchase_orders.first().unwrap();
    let purchase_order =
        xero_rs::purchase_order::get(&client, purchase_order_from_list.purchase_order_id).await?;
    assert_eq!(
        purchase_order_from_list.purchase_order_id,
        purchase_order.purchase_order_id
    );

    Ok(())
}
