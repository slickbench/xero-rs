#[macro_use]
extern crate tracing;

mod test_utils;

use anyhow::Result;
use rust_decimal::Decimal;
use std::env;
use uuid::Uuid;
use xero_rs::{
    KeyPair,
    contact::ContactIdentifier,
    invoice::{Builder, Type},
    line_item::LineAmountType,
};

/// Try to set up a client. Will return None if the required environment variables are not set.
async fn try_setup_client() -> Option<xero_rs::Client> {
    test_utils::do_setup();

    // Check if required environment variables are set
    let client_id = env::var("XERO_CLIENT_ID").ok()?;
    let client_secret = env::var("XERO_CLIENT_SECRET").ok()?;
    let tenant_id_str = env::var("XERO_TENANT_ID").ok()?;

    let tenant_id = match Uuid::parse_str(&tenant_id_str) {
        Ok(id) => id,
        Err(_) => {
            warn!("Invalid XERO_TENANT_ID format");
            return None;
        }
    };

    // Create client with credentials and full scopes
    let client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        xero_rs::Scope::all_accounting(),
    )
    .await
    .ok()?;

    // Set the tenant ID and return the configured client
    client.set_tenant(Some(tenant_id)).await;

    Some(client)
}

#[tokio::test]
async fn test_line_item_with_discount_amount() -> Result<()> {
    // Try to set up the client
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // First get a contact to use
    let contacts = match client.contacts().list().await {
        Ok(contacts) => contacts,
        Err(e) => {
            info!("Skipping test: Could not retrieve contacts: {}", e);
            return Ok(());
        }
    };

    if contacts.is_empty() {
        info!("No contacts found, skipping discount amount test");
        return Ok(());
    }

    let contact_id = contacts[0].contact_id;

    // Create a line item with both discount_rate and discount_amount
    // Note: In practice, you would typically use either discount_rate OR discount_amount, not both
    let line_item_with_discount_rate = xero_rs::line_item::Builder {
        description: Some("Item with percentage discount".to_string()),
        quantity: Some(Decimal::new(200, 2)),     // 2.00
        unit_amount: Some(Decimal::new(5000, 2)), // 50.00
        account_code: Some("200".to_string()),
        tax_type: Some("OUTPUT".to_string()),
        discount_rate: Some(Decimal::new(1000, 2)), // 10.00%
        discount_amount: None,
        ..Default::default()
    };

    let line_item_with_discount_amount = xero_rs::line_item::Builder {
        description: Some("Item with fixed discount amount".to_string()),
        quantity: Some(Decimal::new(300, 2)),     // 3.00
        unit_amount: Some(Decimal::new(7500, 2)), // 75.00
        account_code: Some("200".to_string()),
        tax_type: Some("OUTPUT".to_string()),
        discount_rate: None,
        discount_amount: Some(Decimal::new(2500, 2)), // $25.00 discount
        ..Default::default()
    };

    // Create an invoice with line items having discounts
    let today = time::OffsetDateTime::now_utc().date();
    let due_date = today + time::Duration::days(30);

    let invoice_builder = Builder {
        r#type: Type::AccountsReceivable,
        contact: ContactIdentifier::ID(contact_id),
        line_items: vec![line_item_with_discount_rate, line_item_with_discount_amount],
        date: Some(today),
        due_date: Some(due_date),
        line_amount_types: Some(LineAmountType::Exclusive),
        reference: Some("Testing discount_amount field".to_string()),
        ..Default::default()
    };

    // Create the invoice
    match client.invoices().create(&invoice_builder).await {
        Ok(invoice) => {
            info!(
                "Created invoice with discount amounts: {}",
                invoice.invoice_id
            );

            // Check the line items in the response
            for (i, line_item) in invoice.line_items.iter().enumerate() {
                info!(
                    "Line item {}: {} - Discount Rate: {:?}, Discount Amount: {:?}",
                    i + 1,
                    line_item
                        .description
                        .as_ref()
                        .unwrap_or(&"No description".to_string()),
                    line_item.discount_rate,
                    line_item.discount_amount
                );
            }

            // Check the total discount
            if let Some(total_discount) = invoice.total_discount {
                info!("Total invoice discount: {}", total_discount);
            }
        }
        Err(e) => {
            error!("Could not create invoice with discount amounts: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
