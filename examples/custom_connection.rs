#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::KeyPair;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Get credentials from environment
    let client_id = std::env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret =
        std::env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");

    // Create a client with client credentials and required scopes
    let mut client = xero_rs::Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        xero_rs::scopes![
            xero_rs::ScopeType::AccountingTransactions(xero_rs::Permission::ReadOnly),
            xero_rs::ScopeType::PayrollTimesheets(xero_rs::Permission::ReadWrite),
            xero_rs::ScopeType::PayrollSettings(xero_rs::Permission::ReadWrite), 
            xero_rs::ScopeType::PayrollEmployees(xero_rs::Permission::ReadWrite),
            xero_rs::ScopeType::PayrollPayslip(xero_rs::Permission::ReadWrite),
            xero_rs::ScopeType::PayrollPayruns(xero_rs::Permission::ReadWrite)
        ],
    )
    .await?;

    // Get tenant ID from connections
    let connections = xero_rs::connection::list(&client).await?;
    info!("found client connections: {:#?}", connections);
    let tenant_id = connections.first().expect("No connections found").tenant_id;
    client.set_tenant(Some(tenant_id));

    // Now you can use the client to access the API
    // For example, let's try to list timesheets
    let timesheets = xero_rs::entities::timesheet::Timesheet::list(&client).await?;
    info!("Found {} timesheets", timesheets.len());

    Ok(())
}
