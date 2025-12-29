#[macro_use]
extern crate tracing;

mod test_utils;

use anyhow::Result;
use std::env;
use uuid::Uuid;
use xero_rs::KeyPair;
use xero_rs::entities::account::{AccountStatus, AccountType, Builder, ListParameters};

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
async fn list_accounts() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // List all accounts
    let accounts = client.accounts().list_all().await?;
    info!("Found {} accounts in chart of accounts", accounts.len());
    assert!(!accounts.is_empty(), "Should have at least one account");

    // List with filtering
    let params = ListParameters::builder().with_order("Code ASC");
    let ordered_accounts = client.accounts().list(params).await?;
    info!("Found {} accounts ordered by code", ordered_accounts.len());

    Ok(())
}

#[tokio::test]
async fn get_account() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all accounts first
    let accounts = client.accounts().list_all().await?;

    if !accounts.is_empty() {
        let account_id = accounts[0].account_id;
        let account = client.accounts().get(account_id).await?;
        info!(
            "Got account: {:?} - {} ({:?})",
            account.code, account.name, account.account_type
        );
        assert_eq!(account.account_id, account_id);
    } else {
        info!("No accounts found to test get functionality");
    }

    Ok(())
}

#[tokio::test]
async fn filter_accounts_by_type() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Filter by Revenue type
    let params = ListParameters::builder().with_type(AccountType::Revenue);
    let revenue_accounts = client.accounts().list(params).await?;
    info!("Found {} revenue accounts", revenue_accounts.len());

    // Verify all returned accounts are Revenue type
    for account in &revenue_accounts {
        assert_eq!(
            account.account_type,
            Some(AccountType::Revenue),
            "Expected Revenue type"
        );
    }

    // Filter by Expense type
    let params = ListParameters::builder().with_type(AccountType::Expense);
    let expense_accounts = client.accounts().list(params).await?;
    info!("Found {} expense accounts", expense_accounts.len());

    // Filter by Bank type
    let params = ListParameters::builder().with_type(AccountType::Bank);
    let bank_accounts = client.accounts().list(params).await?;
    info!("Found {} bank accounts", bank_accounts.len());

    Ok(())
}

#[tokio::test]
async fn filter_accounts_by_status() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Filter by Active status
    let params = ListParameters::builder().with_status(AccountStatus::Active);
    let active_accounts = client.accounts().list(params).await?;
    info!("Found {} active accounts", active_accounts.len());

    // Filter by Archived status
    let params = ListParameters::builder().with_status(AccountStatus::Archived);
    let archived_accounts = client.accounts().list(params).await?;
    info!("Found {} archived accounts", archived_accounts.len());

    Ok(())
}

#[tokio::test]
async fn create_update_account() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Create a unique account code using timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let account_code = format!("T{}", timestamp % 100000); // Max 10 chars

    // Create an expense account
    let builder = Builder::new(&account_code, "Test Expense Account", AccountType::Expense)
        .with_description("Test account created by integration tests")
        .with_tax_type("NONE");

    let created_account = match client.accounts().create(&builder).await {
        Ok(account) => {
            info!(
                "Created account: {:?} - {} (ID: {})",
                account.code, account.name, account.account_id
            );
            account
        }
        Err(e) => {
            info!("Could not create account: {}", e);
            return Ok(());
        }
    };

    assert_eq!(created_account.code, Some(account_code.clone()));
    assert_eq!(created_account.name, "Test Expense Account");

    // Update the account
    let update_builder = Builder::new(&account_code, "Updated Test Account", AccountType::Expense)
        .with_description("Updated description");

    match client
        .accounts()
        .update(created_account.account_id, &update_builder)
        .await
    {
        Ok(updated_account) => {
            info!(
                "Updated account: {:?} - {}",
                updated_account.code, updated_account.name
            );
            assert_eq!(updated_account.name, "Updated Test Account");
        }
        Err(e) => {
            info!("Could not update account: {}", e);
        }
    }

    // Clean up - archive the account (can't delete accounts with transactions)
    let archive_builder = Builder::new(&account_code, "Updated Test Account", AccountType::Expense)
        .with_status(AccountStatus::Archived);

    match client
        .accounts()
        .update(created_account.account_id, &archive_builder)
        .await
    {
        Ok(archived_account) => {
            info!("Archived account: {:?}", archived_account.code);
            assert_eq!(archived_account.status, Some(AccountStatus::Archived));
        }
        Err(e) => {
            info!("Could not archive account: {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn account_attachments() -> Result<()> {
    let client = match try_setup_client().await {
        Some(client) => client,
        None => {
            info!("Skipping test: Required environment variables not set");
            return Ok(());
        }
    };

    // Get all accounts first
    let accounts = client.accounts().list_all().await?;

    if !accounts.is_empty() {
        let account_id = accounts[0].account_id;

        // List attachments
        match client.accounts().list_attachments(account_id).await {
            Ok(attachments) => {
                info!("Account has {} attachments", attachments.len());
            }
            Err(e) => {
                info!("Could not list attachments: {}", e);
            }
        }

        // Upload a test attachment
        let test_content = b"Test attachment for account";
        match client
            .accounts()
            .upload_attachment(account_id, "test_account_attachment.txt", test_content)
            .await
        {
            Ok(attachment) => {
                info!("Uploaded attachment: {}", attachment.attachment_id);

                // Download it back
                match client
                    .accounts()
                    .get_attachment(account_id, attachment.attachment_id)
                    .await
                {
                    Ok(data) => {
                        info!("Downloaded attachment with {} bytes", data.len());
                        assert_eq!(data, test_content);
                    }
                    Err(e) => info!("Could not download attachment: {}", e),
                }
            }
            Err(e) => {
                info!("Could not upload attachment: {}", e);
            }
        }
    } else {
        info!("No accounts found to test attachment functionality");
    }

    Ok(())
}
