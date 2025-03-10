#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::{oauth::KeyPair, Client, scope::{Scope, ScopeType, Permission}};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Get credentials from environment
    let client_id = std::env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret =
        std::env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");

    // Create a scope with multiple permissions
    let scope = Scope::from(vec![
        ScopeType::AccountingTransactions(Permission::ReadOnly),
        ScopeType::PayrollTimesheets(Permission::ReadWrite),
        ScopeType::PayrollSettings(Permission::ReadWrite), 
        ScopeType::PayrollEmployees(Permission::ReadWrite),
        ScopeType::PayrollPayslip(Permission::ReadWrite),
        ScopeType::PayrollPayruns(Permission::ReadWrite)
    ]);

    // Create a client with client credentials and required scopes
    let mut client = Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        Some(scope),
    )
    .await?;

    // Get tenant ID from connections
    let connections = xero_rs::entities::connection::list(&client).await?;
    info!("found client connections: {:#?}", connections);
    let tenant_id = connections.first().expect("No connections found").tenant_id;
    client.set_tenant(Some(tenant_id));

    // Now you can use the client to access the API
    // For example, let's try to list timesheets
    let timesheets = client.timesheets().list(None, None).await?;
    info!("Found {} timesheets", timesheets.len());

    Ok(())
}
