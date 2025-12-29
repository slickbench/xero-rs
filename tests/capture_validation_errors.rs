/// Integration test to capture real Xero API validation error responses.
///
/// This test deliberately triggers validation errors to capture the actual JSON
/// responses from the Xero API. These responses are saved as fixtures for use in
/// unit tests.
///
/// Run with: cargo test --test capture_validation_errors -- --ignored --nocapture
///
/// Requires environment variables:
/// - XERO_TENANT_ID
/// - XERO_CLIENT_ID
/// - XERO_CLIENT_SECRET
use std::fs;
use std::path::Path;

use miette::{IntoDiagnostic, Result};
use rust_decimal_macros::dec;
use time::macros::date;
use tracing::info;
use uuid::Uuid;
use xero_rs::Error;
use xero_rs::contact::ContactIdentifier;
use xero_rs::line_item::{self, LineAmountType};
use xero_rs::purchase_order;
use xero_rs::quote::{QuoteBuilder, Status};

mod test_utils;

/// Helper to save error response to fixture file
fn save_fixture(name: &str, content: &str) -> Result<()> {
    let fixtures_dir = Path::new("tests/fixtures");
    fs::create_dir_all(fixtures_dir).into_diagnostic()?;

    let file_path = fixtures_dir.join(format!("{}.json", name));
    fs::write(&file_path, content).into_diagnostic()?;

    info!("Saved fixture to: {}", file_path.display());
    Ok(())
}

#[tokio::test]
#[ignore] // Run explicitly with --ignored
async fn capture_quote_validation_error_missing_contact() -> Result<()> {
    test_utils::do_setup();

    let client = test_utils::create_test_client(Some(test_utils::accounting_scopes())).await?;

    // Get a real contact first
    let contacts = client.contacts().list().await?;
    if contacts.is_empty() {
        info!("No contacts found, skipping test");
        return Ok(());
    }
    let contact_id = contacts[0].contact_id;

    // Create a valid quote first
    let quote_builder = QuoteBuilder {
        contact: ContactIdentifier::ID(contact_id),
        date: date!(2024 - 01 - 01),
        expiry_date: None,
        line_items: vec![],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Test Quote".to_string()),
        summary: None,
        terms: None,
        reference: None,
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Draft),
    };

    // Create the quote
    let created_quote = client.quotes().create(&quote_builder).await;

    if let Ok(created_quote) = created_quote {
        info!("Created quote with ID: {}", created_quote.quote_id);

        // Now try to update it with an invalid contact ID - should trigger validation error
        let invalid_update = QuoteBuilder {
            contact: ContactIdentifier::ID(Uuid::nil()), // Invalid contact!
            date: date!(2024 - 01 - 01),
            expiry_date: None,
            line_items: vec![],
            line_amount_types: LineAmountType::Exclusive,
            title: Some("Updated Test Quote".to_string()),
            summary: None,
            terms: None,
            reference: None,
            currency_code: None,
            branding_theme_id: None,
            quote_id: Some(created_quote.quote_id),
            quote_number: None,
            status: Some(Status::Draft),
        };

        let result = client
            .quotes()
            .update(created_quote.quote_id, &invalid_update)
            .await;

        if let Err(Error::API(api_error)) = result {
            if let xero_rs::error::ErrorType::ValidationException { .. } = api_error.error {
                if let Ok(json) = serde_json::to_string_pretty(&api_error) {
                    save_fixture("quote_validation_missing_contact", &json)?;
                    info!("Successfully captured Quote validation error");
                }
            } else {
                info!("Got non-validation error: {:?}", api_error.error);
            }
        } else {
            info!("Expected validation error but got: {:?}", result);
        }
    } else {
        info!("Failed to create initial quote: {:?}", created_quote);
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn capture_purchase_order_validation_error() -> Result<()> {
    test_utils::do_setup();

    let client = test_utils::create_test_client(Some(test_utils::accounting_scopes())).await?;

    // Try to create a purchase order with an invalid contact ID
    let line_items: Vec<line_item::Builder> = vec![line_item::Builder::new(
        Some("Test Item".to_string()),
        Some(dec!(1.00)),
        Some(dec!(100.00)),
    )];

    // Create PurchaseOrder with invalid contact ID - should trigger validation error
    let po_builder = purchase_order::Builder::new(
        ContactIdentifier::ID(Uuid::nil()), // Invalid contact!
        line_items,
    );

    let result = client.purchase_orders().create(&po_builder).await;

    if let Err(Error::API(api_error)) = result {
        if let xero_rs::error::ErrorType::ValidationException { .. } = api_error.error {
            if let Ok(json) = serde_json::to_string_pretty(&api_error) {
                save_fixture("purchase_order_validation_missing_contact", &json)?;
                info!("Successfully captured PurchaseOrder validation error");
            }
        } else {
            info!("Got non-validation error: {:?}", api_error.error);
        }
    } else {
        info!("Expected validation error but got: {:?}", result);
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn capture_quote_validation_multiple_errors() -> Result<()> {
    test_utils::do_setup();

    let client = test_utils::create_test_client(Some(test_utils::accounting_scopes())).await?;

    // Try to create a quote with invalid data - should trigger validation errors
    let invalid_quote = QuoteBuilder {
        contact: ContactIdentifier::ID(Uuid::nil()), // Invalid contact
        date: date!(1900 - 01 - 01),                 // Very old date might be invalid
        expiry_date: None,
        line_items: vec![], // No line items might be invalid
        line_amount_types: LineAmountType::Exclusive,
        title: None,
        summary: None,
        terms: None,
        reference: None,
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: None,
    };

    let result = client.quotes().create(&invalid_quote).await;

    if let Err(Error::API(api_error)) = result {
        if let xero_rs::error::ErrorType::ValidationException { .. } = api_error.error {
            if let Ok(json) = serde_json::to_string_pretty(&api_error) {
                save_fixture("quote_validation_multiple_errors", &json)?;
                info!("Successfully captured Quote with multiple validation errors");
            }
        } else {
            info!("Got non-validation error: {:?}", api_error.error);
        }
    } else {
        info!("Expected validation error but got: {:?}", result);
    }

    Ok(())
}
