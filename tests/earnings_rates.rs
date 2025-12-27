#[macro_use]
extern crate tracing;

mod test_utils;

use miette::Result;

/// Integration test for the Earnings Rates API
///
/// Tests the earnings_rates().list() method which was previously only tested
/// indirectily through timesheet tests.
///
/// Note: This test requires payroll scopes to be configured in the Xero app.
/// If the app doesn't have payroll permissions, the test will skip gracefully.
#[tokio::test]
async fn list_earnings_rates() -> Result<()> {
    test_utils::do_setup();
    info!("Starting earnings rates list test");

    // Create client with payroll scopes
    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;

    // List earnings rates
    let earnings_rates = match client.earnings_rates().list().await {
        Ok(rates) => rates,
        Err(xero_rs::error::Error::Forbidden(_)) => {
            info!("Payroll scopes not available - skipping earnings rates test");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to list earnings rates: {:?}", e);
            return Err(miette::miette!("Failed to list earnings rates: {:?}", e));
        }
    };

    info!("Found {} earnings rates", earnings_rates.len());

    // If we have earnings rates, verify basic fields are populated
    if let Some(rate) = earnings_rates.first() {
        info!(
            "First earnings rate: {} (ID: {})",
            rate.name, rate.earnings_rate_id
        );
        assert!(!rate.name.is_empty());
    }

    test_utils::do_cleanup().await;
    Ok(())
}
