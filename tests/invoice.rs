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
use xero_rs::invoice::{Builder, ListParameters, Type};
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
    let mut client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        xero_rs::Scope::all_accounting(),
    )
    .await
    .ok()?;

    // Set the tenant ID and return the configured client
    client.set_tenant(Some(tenant_id));

    Some(client)
}

#[tokio::test]
async fn list_invoices() -> Result<()> {
    // Try to set up the client
    let mut client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // List all invoices
    let invoices = client.invoices().list_all().await?;
    info!("Found {} invoices", invoices.len());

    // List with filtering using builder pattern
    let params = ListParameters::builder().with_page(1);
    let filtered_invoices = client.invoices().list(params).await?;
    info!("Found {} invoices with filtering", filtered_invoices.len());

    Ok(())
}

#[tokio::test]
async fn get_invoice() -> Result<()> {
    // Try to set up the client
    let mut client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all invoices first
    let invoices = client.invoices().list_all().await?;

    // If there are invoices, get a specific one
    if !invoices.is_empty() {
        let invoice_id = invoices[0].invoice_id;
        let invoice = client.invoices().get(invoice_id).await?;
        info!(
            "Got invoice #{}: {:?}",
            invoice.invoice_id, invoice.invoice_number
        );
    } else {
        info!("No invoices found to test get functionality");
    }

    Ok(())
}

#[tokio::test]
async fn create_update_invoice() -> Result<()> {
    // Try to set up the client
    let mut client = match try_setup_client().await {
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
        info!("No contacts found, skipping invoice creation test");
        return Ok(());
    }

    let contact_id = contacts[0].contact_id;

    // Create a line item
    let line_item = xero_rs::line_item::Builder {
        description: Some("Test Invoice Item".to_string()),
        quantity: Some(Decimal::new(100, 2)),      // 1.00
        unit_amount: Some(Decimal::new(10000, 2)), // 100.00
        account_code: Some("200".to_string()),
        tax_type: Some("OUTPUT".to_string()),
        ..Default::default()
    };

    // Create a copy for update
    let line_item_for_update = xero_rs::line_item::Builder {
        description: Some("Test Invoice Item".to_string()),
        quantity: Some(Decimal::new(100, 2)),      // 1.00
        unit_amount: Some(Decimal::new(10000, 2)), // 100.00
        account_code: Some("200".to_string()),
        tax_type: Some("OUTPUT".to_string()),
        ..Default::default()
    };

    // Create an invoice
    let invoice_builder = Builder {
        r#type: Type::AccountsReceivable,
        contact: Some(ContactIdentifier::ID(contact_id)),
        line_items: vec![line_item],
        date: Some(date!(2023 - 10 - 01)),
        due_date: Some(date!(2023 - 10 - 31)),
        line_amount_types: Some(LineAmountType::Exclusive),
        invoice_number: Some("INV-TEST-001".to_string()),
        reference: Some("TEST-REF-001".to_string()),
        ..Default::default()
    };

    // Create the invoice
    let created_invoice = match client
        .invoices()
        .create(&invoice_builder, &xero_rs::MutationOptions::default())
        .await
    {
        Ok(invoice) => {
            info!("Created invoice: {}", invoice.invoice_id);
            invoice
        }
        Err(e) => {
            info!("Could not create invoice: {}", e);
            return Ok(());
        }
    };

    // Now update it
    let updated_builder = Builder {
        r#type: Type::AccountsReceivable,
        contact: Some(ContactIdentifier::ID(contact_id)),
        line_items: vec![line_item_for_update],
        date: Some(date!(2023 - 10 - 01)),
        due_date: Some(date!(2023 - 11 - 15)), // Extended due date
        line_amount_types: Some(LineAmountType::Exclusive),
        invoice_number: Some("INV-TEST-001".to_string()),
        reference: Some("TEST-REF-001-UPDATED".to_string()),
        ..Default::default()
    };

    match client
        .invoices()
        .update(created_invoice.invoice_id, &updated_builder)
        .await
    {
        Ok(invoice) => {
            info!(
                "Updated invoice: {} - {:?}",
                invoice.invoice_id, invoice.reference
            );
        }
        Err(e) => {
            info!("Could not update invoice: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn invoice_history() -> Result<()> {
    // Try to set up the client
    let mut client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all invoices first
    let invoices = client.invoices().list_all().await?;

    // If there are invoices, work with the first one
    if !invoices.is_empty() {
        let invoice_id = invoices[0].invoice_id;

        // Add a history record
        match client
            .invoices()
            .create_history(invoice_id, "Testing history creation")
            .await
        {
            Ok(_) => info!("Created history record"),
            Err(e) => info!("Could not create history record: {}", e),
        };

        // Get history records
        match client.invoices().get_history(invoice_id).await {
            Ok(history) => info!("Found {} history records", history.len()),
            Err(e) => info!("Could not get history records: {}", e),
        };
    } else {
        info!("No invoices found to test history functionality");
    }

    Ok(())
}

#[tokio::test]
async fn invoice_pdf() -> Result<()> {
    // Try to set up the client
    let mut client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all invoices first
    let invoices = client.invoices().list_all().await?;

    // If there are invoices, get a PDF for the first one
    if !invoices.is_empty() {
        let invoice_id = invoices[0].invoice_id;

        // Get the PDF
        match client.invoices().get_pdf(invoice_id).await {
            Ok(pdf_data) => {
                info!("Downloaded PDF with {} bytes", pdf_data.len());

                // Optionally save for manual inspection
                let pdf_path = format!("invoice_{}.pdf", invoice_id);
                if let Err(e) = fs::write(&pdf_path, &pdf_data) {
                    info!("Could not save PDF file: {}", e);
                } else {
                    info!("Saved PDF to {}", pdf_path);
                }
            }
            Err(e) => info!("Could not download PDF: {}", e),
        }
    } else {
        info!("No invoices found to test PDF functionality");
    }

    Ok(())
}

#[tokio::test]
async fn invoice_attachments() -> Result<()> {
    // Try to set up the client
    let mut client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all invoices first
    let invoices = client.invoices().list_all().await?;

    // If there are invoices, work with the first one
    if !invoices.is_empty() {
        let invoice_id = invoices[0].invoice_id;

        // Create a sample attachment
        let test_file_content = b"This is a test file for invoice attachment testing";
        let filename = "test_attachment.txt";

        // Upload the attachment
        let _attachment = match client
            .invoices()
            .upload_attachment(invoice_id, filename, test_file_content)
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
        match client.invoices().list_attachments(invoice_id).await {
            Ok(attachments) => {
                info!("Invoice has {} attachments", attachments.len());

                if !attachments.is_empty() {
                    // Get an attachment by ID
                    let attachment_id = attachments[0].attachment_id;
                    match client
                        .invoices()
                        .get_attachment(invoice_id, attachment_id)
                        .await
                    {
                        Ok(data) => info!("Downloaded attachment with {} bytes", data.len()),
                        Err(e) => info!("Could not download attachment: {}", e),
                    };

                    // Get an attachment by filename
                    let filename = &attachments[0].file_name;
                    match client
                        .invoices()
                        .get_attachment_by_filename(invoice_id, filename)
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
                        .invoices()
                        .update_attachment(invoice_id, filename, updated_content)
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
        info!("No invoices found to test attachment functionality");
    }

    Ok(())
}

#[tokio::test]
async fn invoice_online_url() -> Result<()> {
    // Try to set up the client
    let mut client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all invoices first
    let invoices = client.invoices().list_all().await?;

    // If there are invoices, work with the first one
    if !invoices.is_empty() {
        let invoice_id = invoices[0].invoice_id;

        // Try to get the online invoice URL
        match client.invoices().get_online_invoice(invoice_id).await {
            Ok(url) => info!("Got online invoice URL: {}", url),
            Err(e) => info!("Could not get online invoice URL: {}", e),
        };
    } else {
        info!("No invoices found to test online invoice URL functionality");
    }

    Ok(())
}

#[tokio::test]
async fn invoice_email() -> Result<()> {
    // Try to set up the client
    let mut client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all invoices first
    let invoices = client.invoices().list_all().await?;

    // If there are invoices, work with the first one
    if !invoices.is_empty() {
        let _invoice_id = invoices[0].invoice_id;

        // Try to email the invoice
        // Note: Uncomment this when you want to actually send an email
        // match client.invoices().email(_invoice_id).await {
        //     Ok(_) => info!("Invoice email sent successfully"),
        //     Err(e) => info!("Could not send invoice email: {}", e),
        // };
        info!("Email functionality available but not executed in test");
    } else {
        info!("No invoices found to test email functionality");
    }

    Ok(())
}
