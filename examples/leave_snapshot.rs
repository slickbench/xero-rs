//! Read-only leave snapshot check. Prints counts only, never payroll details.
use xero_rs::{Client, KeyPair};

#[tokio::main(flavor = "current_thread")]
async fn main() -> miette::Result<()> {
    let keypair = KeyPair::from_env();
    let client = Client::from_client_credentials(keypair.clone(), None)
        .await
        .map_err(|_| miette::miette!("Xero authentication failed"))?
        .with_auto_refresh(keypair)
        .with_rate_limit(50);
    let applications = client
        .leave_applications()
        .list_all_v2()
        .await
        .map_err(|_| miette::miette!("Leave snapshot failed; verify payroll employee scope"))?;
    println!("Fetched {} leave applications", applications.len());
    let missing_period_dates = applications
        .iter()
        .flat_map(|a| a.leave_periods.iter().flatten())
        .filter(|p| p.pay_period_start_date.is_none() || p.pay_period_end_date.is_none())
        .count();
    println!("Periods with missing dates: {missing_period_dates}");
    if std::env::args().any(|arg| arg == "--verify-refresh") {
        client.clear_access_token_for_testing().await;
        let refreshed = client
            .leave_applications()
            .list_all_v2()
            .await
            .map_err(|_| miette::miette!("Leave read after token refresh failed"))?;
        println!(
            "Token refresh verified; fetched {} applications",
            refreshed.len()
        );
    }
    Ok(())
}
