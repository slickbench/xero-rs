use miette::Result;
use xero_rs::{Client, KeyPair};

mod test_utils;

/// Test that automatic token refresh works when enabled
#[tokio::test]
async fn test_automatic_token_refresh_succeeds() -> Result<()> {
    test_utils::do_setup();

    // Get environment variables for credentials
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();
    let key_pair = KeyPair::new(client_id, Some(client_secret));

    // Create client with auto-refresh enabled (use app default scopes)
    let client = Client::from_client_credentials(key_pair.clone(), None)
        .await
        .map_err(|e| miette::miette!("Failed to create client: {:?}", e))?
        .with_auto_refresh(key_pair);

    // Discover tenant
    let connections = xero_rs::connection::list(&client)
        .await
        .map_err(|e| miette::miette!("Failed to list connections: {:?}", e))?;
    let tenant = connections
        .first()
        .expect("No tenants connected to this app");
    client.set_tenant(Some(tenant.tenant_id)).await;

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
    client.clear_access_token_for_testing().await;
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
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();
    let key_pair = KeyPair::new(client_id, Some(client_secret));

    // Create client WITHOUT auto-refresh (use app default scopes)
    let client = Client::from_client_credentials(key_pair, None)
        .await
        .map_err(|e| miette::miette!("Failed to create client: {:?}", e))?
        .without_auto_refresh(); // Explicitly disable auto-refresh

    // Discover tenant
    let connections = xero_rs::connection::list(&client)
        .await
        .map_err(|e| miette::miette!("Failed to list connections: {:?}", e))?;
    let tenant = connections
        .first()
        .expect("No tenants connected to this app");
    client.set_tenant(Some(tenant.tenant_id)).await;

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
    client.clear_access_token_for_testing().await;
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
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();
    let key_pair = KeyPair::new(client_id.clone(), Some(client_secret.clone()));

    // Create client without auto-refresh (use app default scopes)
    let client = Client::from_client_credentials(key_pair, None)
        .await
        .map_err(|e| miette::miette!("Failed to create client: {:?}", e))?;

    // Discover tenant
    let connections = xero_rs::connection::list(&client)
        .await
        .map_err(|e| miette::miette!("Failed to list connections: {:?}", e))?;
    let tenant = connections
        .first()
        .expect("No tenants connected to this app");
    client.set_tenant(Some(tenant.tenant_id)).await;

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

/// Test that is_token_expiring returns false for a fresh token
#[tokio::test]
async fn test_is_token_expiring_fresh_token() -> Result<()> {
    test_utils::do_setup();

    // Get environment variables for credentials
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();
    let key_pair = KeyPair::new(client_id, Some(client_secret));

    // Create a fresh client
    let client = Client::from_client_credentials(key_pair, None)
        .await
        .map_err(|e| miette::miette!("Failed to create client: {:?}", e))?;

    // A fresh token should NOT be expiring
    let is_expiring = client.is_token_expiring().await;

    assert!(!is_expiring, "Fresh token should not be marked as expiring");
    tracing::info!("is_token_expiring returned {} for fresh token", is_expiring);

    Ok(())
}

/// Test that ensure_valid_token succeeds with a fresh token (no refresh needed)
#[tokio::test]
async fn test_ensure_valid_token_fresh() -> Result<()> {
    test_utils::do_setup();

    // Get environment variables for credentials
    let client_id = std::env::var("XERO_CLIENT_ID").unwrap();
    let client_secret = std::env::var("XERO_CLIENT_SECRET").unwrap();
    let key_pair = KeyPair::new(client_id.clone(), Some(client_secret.clone()));

    // Create client with auto-refresh enabled
    let client = Client::from_client_credentials(key_pair.clone(), None)
        .await
        .map_err(|e| miette::miette!("Failed to create client: {:?}", e))?
        .with_auto_refresh(key_pair);

    // ensure_valid_token should succeed (and do nothing since token is fresh)
    client
        .ensure_valid_token()
        .await
        .map_err(|e| miette::miette!("ensure_valid_token failed: {:?}", e))?;

    tracing::info!("ensure_valid_token succeeded for fresh token");

    // Verify we can still make requests
    let connections = xero_rs::connection::list(&client)
        .await
        .map_err(|e| miette::miette!("Failed to list connections: {:?}", e))?;

    tracing::info!(
        "Listed {} connections after ensure_valid_token",
        connections.len()
    );

    Ok(())
}
