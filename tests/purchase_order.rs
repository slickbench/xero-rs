#[macro_use]
extern crate tracing;

mod test_utils;

use std::env;

use anyhow::Result;
use rust_decimal_macros::dec;
use uuid::Uuid;
use xero_rs::{
    line_item,
    purchase_order::{self, ContactIdentifier},
    KeyPair, Scope,
};

#[tokio::test]
async fn get_purchase_orders() -> Result<()> {
    test_utils::do_setup();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");
    let tenant_id =
        Uuid::parse_str(&env::var("XERO_TENANT_ID").expect("XERO_TENANT_ID must be set"))
            .expect("Invalid XERO_TENANT_ID format");

    // Create client with credentials and scopes
    let mut client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        Some(vec![
            Scope::accounting_transactions_read(),
            Scope::accounting_contacts_read(),
        ]),
    )
    .await?;

    // Set the tenant ID
    client.set_tenant(Some(tenant_id));

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
    test_utils::do_setup();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");
    let tenant_id =
        Uuid::parse_str(&env::var("XERO_TENANT_ID").expect("XERO_TENANT_ID must be set"))
            .expect("Invalid XERO_TENANT_ID format");

    // Create client with credentials and scopes
    let mut client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        Some(vec![
            Scope::accounting_transactions(),
            Scope::accounting_contacts_read(),
        ]),
    )
    .await?;

    // Set the tenant ID
    client.set_tenant(Some(tenant_id));

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
