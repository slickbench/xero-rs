use miette::{IntoDiagnostic, Result};
use tracing::{Level, debug, error, info};
use uuid::Uuid;

use std::sync::Once;

use xero_rs::{Client, KeyPair};

/// Creates a standard test client with the given scopes
#[allow(dead_code)]
pub async fn create_test_client(scopes: Option<xero_rs::Scope>) -> Result<Client> {
    // Get environment variables
    let tenant_id = std::env::var("XERO_TENANT_ID").unwrap();
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();

    info!("Creating client with tenant_id: {}", tenant_id);
    debug!("Using client_id: {}", client_id);
    if let Some(scope) = &scopes {
        debug!("Using scope: {}", scope);
    }

    // Create client
    let client =
        match Client::from_client_credentials(KeyPair::new(client_id, Some(client_secret)), scopes)
            .await
            .into_diagnostic()
        {
            Ok(mut client) => {
                // Set the tenant ID
                client.set_tenant(Some(Uuid::parse_str(&tenant_id).into_diagnostic()?));
                info!("Client created successfully");
                client
            }
            Err(e) => {
                error!("Failed to create client: {:?}", e);
                return Err(miette::miette!("Failed to create client: {:?}", e));
            }
        };

    Ok(client)
}

/// Provides common scopes for payroll tests
#[allow(dead_code)]
pub fn payroll_scopes() -> xero_rs::Scope {
    xero_rs::scopes![
        xero_rs::ScopeType::PayrollTimesheets(xero_rs::Permission::ReadWrite),
        xero_rs::ScopeType::PayrollSettings(xero_rs::Permission::ReadWrite),
        xero_rs::ScopeType::PayrollEmployees(xero_rs::Permission::ReadWrite),
        xero_rs::ScopeType::PayrollPayslip(xero_rs::Permission::ReadWrite),
        xero_rs::ScopeType::PayrollPayruns(xero_rs::Permission::ReadWrite)
    ]
}

/// Provides common scopes for accounting tests
#[allow(dead_code)]
pub fn accounting_scopes() -> xero_rs::Scope {
    xero_rs::Scope::all_accounting()
}

static LOGGING_CONFIGURED: Once = Once::new();

/// Setup before test runs
pub fn do_setup() {
    LOGGING_CONFIGURED.call_once(|| {
        tracing_subscriber::fmt()
            .with_max_level(Level::TRACE)
            .with_test_writer()
            .init()
    });
    info!("Setting up test environment");
}

/// Cleanup after test runs
#[allow(dead_code)]
pub async fn do_cleanup() {
    // Common cleanup code
    info!("Cleaning up test environment");
}
