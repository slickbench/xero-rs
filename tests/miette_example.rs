//! Example test showing how to use the miette integration
use miette::{Result, IntoDiagnostic};

use xero_rs::error::Error;

#[tokio::test]
async fn miette_integration_example() -> Result<()> {
    // Initialize logging for tests
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    
    // Example 1: Direct use of Result with Error type
    // Notice how our Error type is already Diagnostic-compatible
    let result: xero_rs::error::Result<()> = Err(Error::InvalidFilename);
    
    // We can handle errors like this:
    if let Err(e) = result {
        println!("Error with diagnostic info: {:#?}", e);
        // We don't actually want to fail the test here, just demonstrating
    }
    
    // Example 2: Error chaining with Miette
    // When using with miette::Result, we still need .into_diagnostic()
    // for the final conversion since they're different Result types
    if let Err(client_error) = create_client().await.into_diagnostic() {
        println!("Client error with diagnostic info: {:#?}", client_error);
        // We don't actually want to fail the test here, just demonstrating
    }
    
    // Example 3: Using external errors with Miette
    // The ? still requires into_diagnostic() for uuid errors
    // let uuid = uuid::Uuid::parse_str("not-a-uuid").into_diagnostic()?;
    
    Ok(())
}

async fn create_client() -> xero_rs::error::Result<xero_rs::Client> {
    // This function simulates a client creation that could fail
    // Just returning our error type directly
    Err(Error::InvalidEndpoint)
} 