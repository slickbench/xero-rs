#[macro_use]
extern crate tracing;

mod test_utils;

use anyhow::Result;
use std::env;
use std::fs;
use time::macros::date;
use uuid::Uuid;
use xero_rs::{KeyPair, line_item::LineAmountType};
use xero_rs::quote::{QuoteBuilder, Status, ListParameters};

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
    let mut client = client;
    client.set_tenant(Some(tenant_id));
    
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
    let params = ListParameters::builder()
        .with_page(1);
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
    
    // Create a quote
    let quote_builder = QuoteBuilder {
        contact,
        date: date!(2023-10-01),
        expiry_date: Some(date!(2023-10-31)),
        line_items: vec![],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Test Quote".to_string()),
        summary: Some("This is a test quote".to_string()),
        terms: Some("30 days".to_string()),
        reference: Some("TEST-REF-001".to_string()),
        currency_code: Some("USD".to_string()),
        branding_theme_id: None,
        quote_id: None,
        status: Some(Status::Draft),
    };
    
    // Create the quote
    let created_quote = match client.quotes().create(&quote_builder).await {
        Ok(quote) => {
            info!("Created quote: {}", quote.quote_id);
            quote
        },
        Err(e) => {
            info!("Could not create quote: {}", e);
            return Ok(());
        }
    };
    
    // Now update it
    let updated_builder = QuoteBuilder {
        contact: created_quote.contact.clone(),
        date: date!(2023-10-01),
        expiry_date: Some(date!(2023-11-15)), // Extended expiry
        line_items: vec![],
        line_amount_types: LineAmountType::Exclusive,
        title: Some("Updated Test Quote".to_string()),
        summary: Some("This quote has been updated".to_string()),
        terms: Some("30 days".to_string()),
        reference: Some("TEST-REF-001-UPDATED".to_string()),
        currency_code: Some("USD".to_string()),
        branding_theme_id: None,
        quote_id: None,
        status: Some(Status::Draft),
    };
    
    match client.quotes().update(created_quote.quote_id, &updated_builder).await {
        Ok(quote) => {
            info!("Updated quote: {} - {}", quote.quote_id, quote.reference.unwrap_or_default());
        },
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
        match client.quotes().create_history(quote_id, "Testing history creation").await {
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
            },
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
        let _attachment = match client.quotes().upload_attachment(
            quote_id, 
            filename, 
            test_file_content
        ).await {
            Ok(attachment) => {
                info!("Uploaded attachment: {}", attachment.attachment_id);
                Some(attachment)
            },
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
                    match client.quotes().get_attachment(quote_id, attachment_id).await {
                        Ok(data) => info!("Downloaded attachment with {} bytes", data.len()),
                        Err(e) => info!("Could not download attachment: {}", e),
                    };
                    
                    // Get an attachment by filename
                    let filename = &attachments[0].file_name;
                    match client.quotes().get_attachment_by_filename(quote_id, filename).await {
                        Ok(data) => info!("Downloaded attachment by filename with {} bytes", data.len()),
                        Err(e) => info!("Could not download attachment by filename: {}", e),
                    };
                    
                    // Update an attachment
                    let updated_content = b"This file has been updated";
                    match client.quotes().update_attachment(
                        quote_id,
                        filename,
                        updated_content
                    ).await {
                        Ok(attachment) => info!("Updated attachment: {}", attachment.attachment_id),
                        Err(e) => info!("Could not update attachment: {}", e),
                    };
                }
            },
            Err(e) => info!("Could not list attachments: {}", e),
        }
    } else {
        info!("No quotes found to test attachment functionality");
    }
    
    Ok(())
}
