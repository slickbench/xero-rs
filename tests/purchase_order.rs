#[macro_use]
extern crate tracing;

use std::sync::Once;

use anyhow::Result;
use rust_decimal_macros::dec;
use xero_rs::{
    line_item,
    purchase_order::{self, ContactIdentifier},
    KeyPair,
};

static LOGGING_CONFIGURED: Once = Once::new();

fn setup_logging() {
    LOGGING_CONFIGURED.call_once(|| {
        tracing_subscriber::fmt()
            .with_env_filter("info,xero_rs=trace")
            .with_test_writer()
            .init()
    });
}

#[tokio::test]
async fn get_purchase_orders() -> Result<()> {
    setup_logging();
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

#[tokio::test]
async fn create_purchase_order() -> Result<()> {
    setup_logging();
    let client = xero_rs::Client::from_client_credentials(KeyPair::from_env(), None).await?;

    let contact = xero_rs::contact::list(&client)
        .await?
        .into_iter()
        .next()
        .unwrap();

    let description = "test description";
    let quantity = dec!(3.00);
    let unit_amount = dec!(2.00);
    let line_items: Vec<line_item::Builder> = vec![line_item::Builder::new(
        description.to_string(),
        quantity,
        unit_amount,
    )];

    let po_builder =
        purchase_order::Builder::new(ContactIdentifier::ID(contact.contact_id), line_items);
    let created_po = purchase_order::create(&client, &po_builder).await?;

    let po = xero_rs::purchase_order::get(&client, created_po.purchase_order_id).await?;
    assert_eq!(created_po.purchase_order_id, po.purchase_order_id);

    Ok(())
}
