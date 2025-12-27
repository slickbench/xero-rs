#[macro_use]
extern crate tracing;

mod test_utils;

use miette::Result;

/// Integration test for the Employee API
///
/// Tests the employees().list() method which was previously only tested
/// indirectly through timesheet tests.
///
/// Note: This test requires payroll scopes to be configured in the Xero app.
/// If the app doesn't have payroll permissions, the test will skip gracefully.
#[tokio::test]
async fn list_employees() -> Result<()> {
    test_utils::do_setup();
    info!("Starting employee list test");

    // Create client with payroll scopes
    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;

    // List employees
    let employees = match client.employees().list().await {
        Ok(employees) => employees,
        Err(xero_rs::error::Error::Forbidden(_)) => {
            info!("Payroll scopes not available - skipping employee test");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to list employees: {:?}", e);
            return Err(miette::miette!("Failed to list employees: {:?}", e));
        }
    };

    info!("Found {} employees", employees.len());

    // If we have employees, verify basic fields are populated
    if let Some(employee) = employees.first() {
        info!(
            "First employee: {} {} (ID: {})",
            employee.first_name, employee.last_name, employee.employee_id
        );
        assert!(!employee.first_name.is_empty());
        assert!(!employee.last_name.is_empty());
    }

    test_utils::do_cleanup().await;
    Ok(())
}
