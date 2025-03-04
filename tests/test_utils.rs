use miette::{IntoDiagnostic, Result};
use tracing::{debug, error, info};
use uuid::Uuid;

use std::sync::Once;

use xero_rs::{Client, KeyPair, Scope};

/// Creates a standard test client with the given scopes
#[allow(dead_code)]
pub async fn create_test_client(scopes: Vec<Scope>) -> Result<Client> {
    // Get environment variables
    let tenant_id = std::env::var("XERO_TENANT_ID").unwrap();
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();

    info!("Creating client with tenant_id: {}", tenant_id);
    debug!("Using client_id: {}", client_id);
    debug!("Using scopes: {:?}", scopes);

    // Create client
    let client = match Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        Some(scopes),
    )
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
pub fn payroll_scopes() -> Vec<Scope> {
    vec![
        Scope::payroll_timesheets(),
        Scope::payroll_settings(),
        Scope::payroll_employees(),
        Scope::payroll_payslip(),
        Scope::payroll_payruns(),
    ]
}

/// Provides common scopes for accounting tests
#[allow(dead_code)]
pub fn accounting_scopes() -> Vec<Scope> {
    vec![
        Scope::accounting_transactions(),
        Scope::accounting_contacts(),
        Scope::accounting_settings(),
    ]
}

static LOGGING_CONFIGURED: Once = Once::new();

/// Setup before test runs
pub fn do_setup() {
    LOGGING_CONFIGURED.call_once(|| tracing_subscriber::fmt().with_test_writer().init());
    info!("Setting up test environment");
}

/// Cleanup after test runs
#[allow(dead_code)]
pub async fn do_cleanup() {
    // Common cleanup code
    info!("Cleaning up test environment");
}
