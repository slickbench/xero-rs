use miette::Result;
use uuid::Uuid;
use xero_rs::{Client, KeyPair};

mod test_utils;

/// Test that automatic token refresh works when enabled
#[tokio::test]
async fn test_automatic_token_refresh_succeeds() -> Result<()> {
    test_utils::do_setup();

    // Get environment variables for credentials
    let tenant_id = std::env::var("XERO_TENANT_ID").unwrap();
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();
    let key_pair = KeyPair::new(client_id, Some(client_secret));

    // Create client with auto-refresh enabled
    let mut client =
        Client::from_client_credentials(key_pair.clone(), Some(xero_rs::Scope::all_accounting()))
            .await
            .map_err(|e| miette::miette!("Failed to create client: {:?}", e))?
            .with_auto_refresh(key_pair);

    // Set the tenant ID
    client.set_tenant(Some(Uuid::parse_str(&tenant_id).unwrap()));

    // First request should succeed
    let invoices1 = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await;

    assert!(invoices1.is_ok(), "First request should succeed");
    tracing::info!(
        "First request succeeded with {} invoices",
        invoices1.unwrap().len()
    );

    // Clear the access token to simulate token expiry
    // We'll do this by directly setting an invalid token (this is a hack for testing)
    // In a real scenario, the token would expire naturally
    client.clear_access_token_for_testing();
    tracing::info!("Access token cleared (set to invalid)");

    // Second request should trigger automatic refresh and succeed
    let invoices2 = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await;
    tracing::info!("Second request: {:?}", invoices2);

    assert!(
        invoices2.is_ok(),
        "Second request should succeed after automatic refresh"
    );
    tracing::info!(
        "Second request succeeded with {} invoices after auto-refresh",
        invoices2.unwrap().len()
    );

    Ok(())
}

/// Test that requests fail when auto-refresh is disabled
#[tokio::test]
async fn test_without_auto_refresh_fails() -> Result<()> {
    test_utils::do_setup();

    // Get environment variables for credentials
    let tenant_id = std::env::var("XERO_TENANT_ID").unwrap();
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();
    let key_pair = KeyPair::new(client_id, Some(client_secret));

    // Create client WITHOUT auto-refresh
    let mut client =
        Client::from_client_credentials(key_pair, Some(xero_rs::Scope::all_accounting()))
            .await
            .map_err(|e| miette::miette!("Failed to create client: {:?}", e))?
            .without_auto_refresh(); // Explicitly disable auto-refresh

    // Set the tenant ID
    client.set_tenant(Some(Uuid::parse_str(&tenant_id).unwrap()));

    // First request should succeed
    let invoices1 = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await;

    assert!(invoices1.is_ok(), "First request should succeed");
    tracing::info!(
        "First request succeeded with {} invoices",
        invoices1.unwrap().len()
    );

    // Clear the access token to simulate token expiry
    client.clear_access_token_for_testing();
    tracing::info!("Access token cleared (set to invalid)");

    // Second request should fail because auto-refresh is disabled
    let invoices2 = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await;

    assert!(
        invoices2.is_err(),
        "Second request should fail without auto-refresh"
    );

    // Verify it's an unauthorized error
    if let Err(xero_rs::error::Error::API(ref api_err)) = invoices2 {
        assert!(
            matches!(
                api_err.error,
                xero_rs::error::ErrorType::UnauthorisedException
            ),
            "Should be an UnauthorisedException"
        );
        tracing::info!("Second request failed as expected with UnauthorisedException");
    } else {
        panic!(
            "Expected API UnauthorisedException error, got: {:?}",
            invoices2
        );
    }

    Ok(())
}

/// Test that manual refresh still works even when auto-refresh is disabled
#[tokio::test]
async fn test_manual_refresh_still_works() -> Result<()> {
    test_utils::do_setup();

    // Get environment variables for credentials
    let tenant_id = std::env::var("XERO_TENANT_ID").unwrap();
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();
    let key_pair = KeyPair::new(client_id.clone(), Some(client_secret.clone()));

    // Create client without auto-refresh
    let mut client =
        Client::from_client_credentials(key_pair, Some(xero_rs::Scope::all_accounting()))
            .await
            .map_err(|e| miette::miette!("Failed to create client: {:?}", e))?;

    // Set the tenant ID
    client.set_tenant(Some(Uuid::parse_str(&tenant_id).unwrap()));

    // First request should succeed
    let invoices1 = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await;

    assert!(invoices1.is_ok(), "First request should succeed");
    tracing::info!("First request succeeded");

    // Manually refresh the token
    let refresh_key_pair = KeyPair::new(client_id, Some(client_secret));
    client
        .refresh_access_token(refresh_key_pair)
        .await
        .map_err(|e| miette::miette!("Manual refresh failed: {:?}", e))?;

    tracing::info!("Manual token refresh succeeded");

    // Request after manual refresh should succeed
    let invoices2 = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await;

    assert!(
        invoices2.is_ok(),
        "Request after manual refresh should succeed"
    );
    tracing::info!("Request after manual refresh succeeded");

    Ok(())
}
