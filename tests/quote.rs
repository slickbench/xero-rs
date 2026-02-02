#[macro_use]
extern crate tracing;

mod test_utils;

use anyhow::Result;
use rust_decimal::Decimal;
use std::env;
use std::fs;
use time::macros::date;
use uuid::Uuid;
use xero_rs::contact::ContactIdentifier;
use xero_rs::quote::{ListParameters, QuoteBuilder, Status};
use xero_rs::{KeyPair, line_item::LineAmountType};

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
async fn list_quotes() -> Result<()> {
    // Try to set up the client
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // List all quotes
    let quotes = client.quotes().list_all().await?;
    info!("Found {} quotes", quotes.len());

    // List with filtering using builder pattern
    let params = ListParameters::builder().with_page(1);
    let filtered_quotes = client.quotes().list(params).await?;
    info!("Found {} quotes with filtering", filtered_quotes.len());

    Ok(())
}

#[tokio::test]
async fn get_quote() -> Result<()> {
    // Try to set up the client
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all quotes first
    let quotes = client.quotes().list_all().await?;

    // If there are quotes, get a specific one
    if !quotes.is_empty() {
        let quote_id = quotes[0].quote_id;
        let quote = client.quotes().get(quote_id).await?;
        info!("Got quote #{}: {}", quote.quote_id, quote.quote_number);
    } else {
        info!("No quotes found to test get functionality");
    }

    Ok(())
}

#[tokio::test]
async fn create_update_quote() -> Result<()> {
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
        info!("No contacts found, skipping quote creation test");
        return Ok(());
    }

    let contact = contacts[0].clone();
    let contact_id = contact.contact_id;

    // Create a quote
    let quote_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2023 - 10 - 01),
        expiry_date: Some(date!(2023 - 10 - 31)),
        line_items: vec![],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Test Quote".to_string()),
        summary: Some("This is a test quote".to_string()),
        terms: Some("30 days".to_string()),
        reference: Some("TEST-REF-001".to_string()),
        currency_code: Some("USD".to_string()),
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Draft),
    };

    // Create the quote
    let created_quote = match client.quotes().create(&quote_builder).await {
        Ok(quote) => {
            info!("Created quote: {}", quote.quote_id);
            quote
        }
        Err(e) => {
            info!("Could not create quote: {}", e);
            return Ok(());
        }
    };

    // Now update it
    let updated_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2023 - 10 - 01),
        expiry_date: Some(date!(2023 - 11 - 15)), // Extended expiry
        line_items: vec![],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Updated Test Quote".to_string()),
        summary: Some("This quote has been updated".to_string()),
        terms: Some("30 days".to_string()),
        reference: Some("TEST-REF-001-UPDATED".to_string()),
        currency_code: Some("USD".to_string()),
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Draft),
    };

    match client
        .quotes()
        .update(created_quote.quote_id, &updated_builder)
        .await
    {
        Ok(quote) => {
            info!(
                "Updated quote: {} - {}",
                quote.quote_id,
                quote.reference.unwrap_or_default()
            );
        }
        Err(e) => {
            info!("Could not update quote: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn quote_history() -> Result<()> {
    // Try to set up the client
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all quotes first
    let quotes = client.quotes().list_all().await?;

    // If there are quotes, work with the first one
    if !quotes.is_empty() {
        let quote_id = quotes[0].quote_id;

        // Add a history record
        match client
            .quotes()
            .create_history(quote_id, "Testing history creation")
            .await
        {
            Ok(_) => info!("Created history record"),
            Err(e) => info!("Could not create history record: {}", e),
        };

        // Get history records
        match client.quotes().get_history(quote_id).await {
            Ok(history) => info!("Found {} history records", history.len()),
            Err(e) => info!("Could not get history records: {}", e),
        };
    } else {
        info!("No quotes found to test history functionality");
    }

    Ok(())
}

#[tokio::test]
async fn quote_pdf() -> Result<()> {
    // Try to set up the client
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all quotes first
    let quotes = client.quotes().list_all().await?;

    // If there are quotes, get a PDF for the first one
    if !quotes.is_empty() {
        let quote_id = quotes[0].quote_id;

        // Get the PDF
        match client.quotes().get_pdf(quote_id).await {
            Ok(pdf_data) => {
                info!("Downloaded PDF with {} bytes", pdf_data.len());

                // Optionally save for manual inspection
                let pdf_path = format!("quote_{}.pdf", quote_id);
                if let Err(e) = fs::write(&pdf_path, &pdf_data) {
                    info!("Could not save PDF file: {}", e);
                } else {
                    info!("Saved PDF to {}", pdf_path);
                }
            }
            Err(e) => info!("Could not download PDF: {}", e),
        }
    } else {
        info!("No quotes found to test PDF functionality");
    }

    Ok(())
}

#[tokio::test]
async fn quote_attachments() -> Result<()> {
    // Try to set up the client
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all quotes first
    let quotes = client.quotes().list_all().await?;

    // If there are quotes, work with the first one
    if !quotes.is_empty() {
        let quote_id = quotes[0].quote_id;

        // Create a sample attachment
        let test_file_content = b"This is a test file for quote attachment testing";
        let filename = "test_attachment.txt";

        // Upload the attachment
        let _attachment = match client
            .quotes()
            .upload_attachment(quote_id, filename, test_file_content)
            .await
        {
            Ok(attachment) => {
                info!("Uploaded attachment: {}", attachment.attachment_id);
                Some(attachment)
            }
            Err(e) => {
                info!("Could not upload attachment: {}", e);
                None
            }
        };

        // List attachments
        match client.quotes().list_attachments(quote_id).await {
            Ok(attachments) => {
                info!("Quote has {} attachments", attachments.len());

                if !attachments.is_empty() {
                    // Get an attachment by ID
                    let attachment_id = attachments[0].attachment_id;
                    match client
                        .quotes()
                        .get_attachment(quote_id, attachment_id)
                        .await
                    {
                        Ok(data) => info!("Downloaded attachment with {} bytes", data.len()),
                        Err(e) => info!("Could not download attachment: {}", e),
                    };

                    // Get an attachment by filename
                    let filename = &attachments[0].file_name;
                    match client
                        .quotes()
                        .get_attachment_by_filename(quote_id, filename)
                        .await
                    {
                        Ok(data) => info!(
                            "Downloaded attachment by filename with {} bytes",
                            data.len()
                        ),
                        Err(e) => info!("Could not download attachment by filename: {}", e),
                    };

                    // Update an attachment
                    let updated_content = b"This file has been updated";
                    match client
                        .quotes()
                        .update_attachment(quote_id, filename, updated_content)
                        .await
                    {
                        Ok(attachment) => info!("Updated attachment: {}", attachment.attachment_id),
                        Err(e) => info!("Could not update attachment: {}", e),
                    };
                }
            }
            Err(e) => info!("Could not list attachments: {}", e),
        }
    } else {
        info!("No quotes found to test attachment functionality");
    }

    Ok(())
}

#[tokio::test]
async fn create_quote_with_line_items() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get a contact to use
    let contacts = match client.contacts().list().await {
        Ok(contacts) => contacts,
        Err(e) => {
            info!("Skipping test: Could not retrieve contacts: {}", e);
            return Ok(());
        }
    };

    if contacts.is_empty() {
        info!("No contacts found, skipping quote creation test");
        return Ok(());
    }

    let contact_id = contacts[0].contact_id;

    // Create line items
    let line_item_1 = xero_rs::line_item::Builder {
        description: Some("Consulting Services".to_string()),
        quantity: Some(Decimal::new(500, 2)),      // 5.00 hours
        unit_amount: Some(Decimal::new(15000, 2)), // $150.00/hour
        account_code: Some("200".to_string()),
        ..Default::default()
    };

    let line_item_2 = xero_rs::line_item::Builder {
        description: Some("Software License".to_string()),
        quantity: Some(Decimal::new(100, 2)),      // 1.00
        unit_amount: Some(Decimal::new(50000, 2)), // $500.00
        account_code: Some("200".to_string()),
        ..Default::default()
    };

    let line_item_3 = xero_rs::line_item::Builder {
        description: Some("Support Package".to_string()),
        quantity: Some(Decimal::new(100, 2)),      // 1.00
        unit_amount: Some(Decimal::new(25000, 2)), // $250.00
        account_code: Some("200".to_string()),
        ..Default::default()
    };

    let quote_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2024 - 01 - 15),
        expiry_date: Some(date!(2024 - 02 - 15)),
        line_items: vec![line_item_1, line_item_2, line_item_3],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Project Proposal".to_string()),
        summary: Some("Comprehensive project services quote".to_string()),
        terms: Some("Net 30".to_string()),
        reference: Some("QUOTE-LINE-ITEMS-TEST".to_string()),
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Draft),
    };

    match client.quotes().create(&quote_builder).await {
        Ok(quote) => {
            info!(
                "Created quote {} with {} line items",
                quote.quote_id,
                quote.line_items.len()
            );
            assert_eq!(quote.line_items.len(), 3, "Expected 3 line items");

            // Verify totals are calculated (5*150 + 1*500 + 1*250 = 1500)
            let expected_subtotal = Decimal::new(150000, 2); // $1500.00
            assert_eq!(
                quote.sub_total, expected_subtotal,
                "Subtotal should be $1500.00"
            );

            info!(
                "Quote totals - SubTotal: {}, Tax: {}, Total: {}",
                quote.sub_total, quote.total_tax, quote.total
            );
        }
        Err(e) => {
            info!("Could not create quote with line items: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn update_quote_line_items() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get a contact
    let contacts = match client.contacts().list().await {
        Ok(contacts) => contacts,
        Err(e) => {
            info!("Skipping test: Could not retrieve contacts: {}", e);
            return Ok(());
        }
    };

    if contacts.is_empty() {
        info!("No contacts found, skipping test");
        return Ok(());
    }

    let contact_id = contacts[0].contact_id;

    // Create initial quote with one line item
    let initial_line_item = xero_rs::line_item::Builder {
        description: Some("Initial Service".to_string()),
        quantity: Some(Decimal::new(100, 2)),      // 1.00
        unit_amount: Some(Decimal::new(10000, 2)), // $100.00
        account_code: Some("200".to_string()),
        ..Default::default()
    };

    let quote_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2024 - 01 - 15),
        expiry_date: Some(date!(2024 - 02 - 15)),
        line_items: vec![initial_line_item],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Quote for Update Test".to_string()),
        summary: None,
        terms: None,
        reference: Some("QUOTE-UPDATE-LINE-ITEMS".to_string()),
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Draft),
    };

    let created_quote = match client.quotes().create(&quote_builder).await {
        Ok(quote) => {
            info!(
                "Created quote {} with {} line item(s)",
                quote.quote_id,
                quote.line_items.len()
            );
            quote
        }
        Err(e) => {
            info!("Could not create quote: {}", e);
            return Ok(());
        }
    };

    assert_eq!(created_quote.line_items.len(), 1);

    // Update: Replace with two new line items (removing original, adding new)
    let new_line_item_1 = xero_rs::line_item::Builder {
        description: Some("Updated Service A".to_string()),
        quantity: Some(Decimal::new(200, 2)),     // 2.00
        unit_amount: Some(Decimal::new(7500, 2)), // $75.00
        account_code: Some("200".to_string()),
        ..Default::default()
    };

    let new_line_item_2 = xero_rs::line_item::Builder {
        description: Some("Updated Service B".to_string()),
        quantity: Some(Decimal::new(300, 2)),     // 3.00
        unit_amount: Some(Decimal::new(5000, 2)), // $50.00
        account_code: Some("200".to_string()),
        ..Default::default()
    };

    let update_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2024 - 01 - 15),
        expiry_date: Some(date!(2024 - 02 - 28)), // Extended expiry
        line_items: vec![new_line_item_1, new_line_item_2],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Quote for Update Test - Revised".to_string()),
        summary: Some("Updated with new line items".to_string()),
        terms: None,
        reference: Some("QUOTE-UPDATE-LINE-ITEMS-REV".to_string()),
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Draft),
    };

    match client
        .quotes()
        .update(created_quote.quote_id, &update_builder)
        .await
    {
        Ok(updated_quote) => {
            info!(
                "Updated quote {} now has {} line items",
                updated_quote.quote_id,
                updated_quote.line_items.len()
            );
            assert_eq!(
                updated_quote.line_items.len(),
                2,
                "Expected 2 line items after update"
            );

            // Verify new subtotal (2*75 + 3*50 = 300)
            let expected_subtotal = Decimal::new(30000, 2); // $300.00
            assert_eq!(
                updated_quote.sub_total, expected_subtotal,
                "Subtotal should be $300.00"
            );

            info!(
                "Updated quote totals - SubTotal: {}, Total: {}",
                updated_quote.sub_total, updated_quote.total
            );
        }
        Err(e) => {
            info!("Could not update quote line items: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn quote_status_transitions() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get a contact
    let contacts = match client.contacts().list().await {
        Ok(contacts) => contacts,
        Err(e) => {
            info!("Skipping test: Could not retrieve contacts: {}", e);
            return Ok(());
        }
    };

    if contacts.is_empty() {
        info!("No contacts found, skipping test");
        return Ok(());
    }

    let contact_id = contacts[0].contact_id;

    // Create a draft quote with at least one line item (required for status transitions)
    let line_item = xero_rs::line_item::Builder {
        description: Some("Status Test Service".to_string()),
        quantity: Some(Decimal::new(100, 2)),
        unit_amount: Some(Decimal::new(10000, 2)),
        account_code: Some("200".to_string()),
        ..Default::default()
    };

    let quote_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2024 - 01 - 15),
        expiry_date: Some(date!(2024 - 02 - 15)),
        line_items: vec![line_item.clone()],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Status Transition Test".to_string()),
        summary: None,
        terms: None,
        reference: Some("QUOTE-STATUS-TEST".to_string()),
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Draft),
    };

    let created_quote = match client.quotes().create(&quote_builder).await {
        Ok(quote) => {
            info!("Created draft quote: {}", quote.quote_id);
            assert!(
                matches!(quote.status, Status::Draft),
                "Quote should be in Draft status"
            );
            quote
        }
        Err(e) => {
            info!("Could not create quote: {}", e);
            return Ok(());
        }
    };

    // Transition Draft -> Sent
    let sent_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2024 - 01 - 15),
        expiry_date: Some(date!(2024 - 02 - 15)),
        line_items: vec![line_item],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Status Transition Test".to_string()),
        summary: None,
        terms: None,
        reference: Some("QUOTE-STATUS-TEST".to_string()),
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Sent),
    };

    match client
        .quotes()
        .update(created_quote.quote_id, &sent_builder)
        .await
    {
        Ok(sent_quote) => {
            info!("Transitioned quote {} to Sent status", sent_quote.quote_id);
            assert!(
                matches!(sent_quote.status, Status::Sent),
                "Quote should now be in Sent status"
            );
        }
        Err(e) => {
            info!("Could not transition quote to Sent: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn list_quotes_with_filters() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Test status filter
    let draft_params = ListParameters::builder().with_status(Status::Draft);
    match client.quotes().list(draft_params).await {
        Ok(drafts) => info!("Found {} draft quotes", drafts.len()),
        Err(e) => info!("Could not filter by draft status: {}", e),
    }

    let sent_params = ListParameters::builder().with_status(Status::Sent);
    match client.quotes().list(sent_params).await {
        Ok(sent) => info!("Found {} sent quotes", sent.len()),
        Err(e) => info!("Could not filter by sent status: {}", e),
    }

    // Test date range filter
    let date_params = ListParameters::builder()
        .with_date_from(date!(2024 - 01 - 01))
        .with_date_to(date!(2024 - 12 - 31));
    match client.quotes().list(date_params).await {
        Ok(quotes) => info!("Found {} quotes in 2024 date range", quotes.len()),
        Err(e) => info!("Could not filter by date range: {}", e),
    }

    // Test expiry date filter
    let expiry_params = ListParameters::builder()
        .with_expiry_date_from(date!(2024 - 01 - 01))
        .with_expiry_date_to(date!(2024 - 12 - 31));
    match client.quotes().list(expiry_params).await {
        Ok(quotes) => info!("Found {} quotes with 2024 expiry dates", quotes.len()),
        Err(e) => info!("Could not filter by expiry date range: {}", e),
    }

    // Test contact filter (if we have quotes and contacts)
    let contacts = client.contacts().list().await.unwrap_or_default();
    if !contacts.is_empty() {
        let contact_params = ListParameters::builder().with_contact_id(contacts[0].contact_id);
        match client.quotes().list(contact_params).await {
            Ok(quotes) => info!(
                "Found {} quotes for contact {}",
                quotes.len(),
                contacts[0].contact_id
            ),
            Err(e) => info!("Could not filter by contact: {}", e),
        }
    }

    // Test ordering
    let ordered_params = ListParameters::builder().with_order("Date DESC");
    match client.quotes().list(ordered_params).await {
        Ok(quotes) => info!("Found {} quotes ordered by date DESC", quotes.len()),
        Err(e) => info!("Could not order quotes: {}", e),
    }

    Ok(())
}

#[tokio::test]
async fn update_quote_number_prefix() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get a contact
    let contacts = match client.contacts().list().await {
        Ok(contacts) => contacts,
        Err(e) => {
            info!("Skipping test: Could not retrieve contacts: {}", e);
            return Ok(());
        }
    };

    if contacts.is_empty() {
        info!("No contacts found, skipping test");
        return Ok(());
    }

    let contact_id = contacts[0].contact_id;

    // Create a draft quote with a line item
    let line_item = xero_rs::line_item::Builder {
        description: Some("Quote Number Test Service".to_string()),
        quantity: Some(Decimal::new(100, 2)),
        unit_amount: Some(Decimal::new(10000, 2)),
        account_code: Some("200".to_string()),
        ..Default::default()
    };

    let quote_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2024 - 01 - 15),
        expiry_date: Some(date!(2024 - 02 - 15)),
        line_items: vec![line_item.clone()],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Quote Number Prefix Test".to_string()),
        summary: None,
        terms: None,
        reference: Some("QUOTE-NUMBER-TEST".to_string()),
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: None,
        status: Some(Status::Draft),
    };

    let created_quote = match client.quotes().create(&quote_builder).await {
        Ok(quote) => {
            info!("Created quote with number: {}", quote.quote_number);
            quote
        }
        Err(e) => {
            info!("Could not create quote: {}", e);
            return Ok(());
        }
    };

    // Update the quote with a new quote number prefix (e.g., QU-0001 -> F-QU-0001)
    let original_number = &created_quote.quote_number;
    let new_quote_number = format!("F-{}", original_number);

    let update_builder = QuoteBuilder {
        contact: Some(ContactIdentifier::ID(contact_id)),
        date: date!(2024 - 01 - 15),
        expiry_date: Some(date!(2024 - 02 - 15)),
        line_items: vec![line_item],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Quote Number Prefix Test".to_string()),
        summary: None,
        terms: None,
        reference: Some("QUOTE-NUMBER-TEST".to_string()),
        currency_code: None,
        branding_theme_id: None,
        quote_id: None,
        quote_number: Some(new_quote_number.clone()),
        status: Some(Status::Draft),
    };

    match client
        .quotes()
        .update(created_quote.quote_id, &update_builder)
        .await
    {
        Ok(updated_quote) => {
            info!(
                "Updated quote number from {} to {}",
                original_number, updated_quote.quote_number
            );
            assert_eq!(
                updated_quote.quote_number, new_quote_number,
                "Quote number should be updated to new prefix"
            );
        }
        Err(e) => {
            info!("Could not update quote number: {}", e);
        }
    }

    Ok(())
}
