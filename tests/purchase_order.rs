#[macro_use]
extern crate tracing;

mod test_utils;

use std::env;

use anyhow::Result;
use rust_decimal_macros::dec;
use uuid::Uuid;
use xero_rs::{
    KeyPair,
    contact::ContactIdentifier,
    line_item,
    purchase_order::{self},
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

    // Create client with credentials and scopes directly
    let client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        xero_rs::scopes![
            xero_rs::ScopeType::AccountingTransactions(xero_rs::Permission::ReadOnly),
            xero_rs::ScopeType::AccountingContacts(xero_rs::Permission::ReadOnly)
        ],
    )
    .await?;

    // Set the tenant ID
    client.set_tenant(Some(tenant_id)).await;

    // Use the new method-based API
    let purchase_orders = client.purchase_orders().list().await?;
    debug!("found {:?} purchase_orders", purchase_orders.len());

    if !purchase_orders.is_empty() {
        let purchase_order_from_list = purchase_orders.first().unwrap();
        let purchase_order = client
            .purchase_orders()
            .get(purchase_order_from_list.purchase_order_id)
            .await?;
        assert_eq!(
            purchase_order_from_list.purchase_order_id,
            purchase_order.purchase_order_id
        );
    }

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

    // Create client with credentials and scopes directly
    let client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        xero_rs::scopes![
            xero_rs::ScopeType::AccountingTransactions(xero_rs::Permission::ReadWrite),
            xero_rs::ScopeType::AccountingContacts(xero_rs::Permission::ReadOnly)
        ],
    )
    .await?;

    // Set the tenant ID
    client.set_tenant(Some(tenant_id)).await;

    // Use the new method-based API
    let contact = client.contacts().list().await?.into_iter().next().unwrap();

    let description = "test description";
    let quantity = dec!(3.00);
    let unit_amount = dec!(2.00);
    let line_items: Vec<line_item::Builder> = vec![line_item::Builder::new(
        Some(description.to_string()),
        Some(quantity),
        Some(unit_amount),
    )];

    let po_builder =
        purchase_order::Builder::new(ContactIdentifier::ID(contact.contact_id), line_items);
    let created_po = client.purchase_orders().create(&po_builder).await?;

    let po = client
        .purchase_orders()
        .get(created_po.purchase_order_id)
        .await?;
    assert_eq!(created_po.purchase_order_id, po.purchase_order_id);

    Ok(())
}

#[tokio::test]
async fn update_purchase_order() -> Result<()> {
    test_utils::do_setup();

    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");
    let tenant_id =
        Uuid::parse_str(&env::var("XERO_TENANT_ID").expect("XERO_TENANT_ID must be set"))
            .expect("Invalid XERO_TENANT_ID format");

    let client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        xero_rs::scopes![
            xero_rs::ScopeType::AccountingTransactions(xero_rs::Permission::ReadWrite),
            xero_rs::ScopeType::AccountingContacts(xero_rs::Permission::ReadOnly)
        ],
    )
    .await?;

    client.set_tenant(Some(tenant_id)).await;

    let contact = client.contacts().list().await?.into_iter().next().unwrap();
    let line_item_builder = line_item::Builder::new(
        Some("update test".to_string()),
        Some(dec!(1.00)),
        Some(dec!(2.50)),
    );
    let po_builder = purchase_order::Builder::new(
        ContactIdentifier::ID(contact.contact_id),
        vec![line_item_builder.clone()],
    );
    let created_po = client.purchase_orders().create(&po_builder).await?;

    let update_line_items: Vec<line_item::Builder> = created_po
        .line_items
        .iter()
        .cloned()
        .map(|item| item.into_builder())
        .collect();
    let mut update_builder = purchase_order::Builder::new(
        ContactIdentifier::ID(created_po.contact.contact_id),
        update_line_items,
    );
    update_builder.attention_to = Some("Updated Approver".to_string());
    update_builder.purchase_order_id = Some(created_po.purchase_order_id);

    let updated_po = client
        .purchase_orders()
        .update(created_po.purchase_order_id, &update_builder)
        .await?;
    assert_eq!(
        updated_po.attention_to,
        Some("Updated Approver".to_string())
    );

    let fetched_po = client
        .purchase_orders()
        .get(created_po.purchase_order_id)
        .await?;
    assert_eq!(
        fetched_po.attention_to,
        Some("Updated Approver".to_string())
    );

    Ok(())
}
