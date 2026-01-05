//! Integration tests for Leave Applications API
//!
//! These tests require a valid Xero API connection with payroll permissions.

use tracing::{error, info};

mod test_utils;

use xero_rs::payroll::leave_application::{LeavePeriodStatus, ListParameters};

#[tokio::test]
async fn test_list_leave_applications() -> miette::Result<()> {
    test_utils::do_setup();
    info!("Starting leave applications list test");

    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;

    // Gracefully handle missing payroll scopes
    match client.leave_applications().list_all().await {
        Ok(leave_apps) => {
            info!("Found {} approved leave applications", leave_apps.len());

            for app in leave_apps.iter().take(3) {
                info!(
                    "Leave: ID={}, Employee={}, Start={}, End={}",
                    app.leave_application_id, app.employee_id, app.start_date, app.end_date
                );

                // Check leave periods if present
                if let Some(periods) = &app.leave_periods {
                    for period in periods {
                        info!(
                            "  Period: Units={:?}, Status={:?}",
                            period.number_of_units, period.leave_period_status
                        );
                    }
                }
            }
        }
        Err(xero_rs::error::Error::Forbidden(_)) => {
            info!("Payroll scopes not available, skipping test");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to list leave applications: {:?}", e);
            return Err(miette::miette!(
                "Failed to list leave applications: {:?}",
                e
            ));
        }
    }

    test_utils::do_cleanup().await;
    info!("Leave applications list test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_list_leave_applications_v2() -> miette::Result<()> {
    test_utils::do_setup();
    info!("Starting leave applications v2 list test (all statuses)");

    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;

    // Test v2 endpoint which returns all statuses including REQUESTED and REJECTED
    match client.leave_applications().list_v2(None, None).await {
        Ok(leave_apps) => {
            info!(
                "Found {} leave applications (including pending/rejected)",
                leave_apps.len()
            );

            // Group by status
            let mut scheduled = 0;
            let mut processed = 0;
            let mut requested = 0;
            let mut rejected = 0;

            for app in &leave_apps {
                if let Some(periods) = &app.leave_periods {
                    for period in periods {
                        match period.leave_period_status {
                            Some(LeavePeriodStatus::Scheduled) => scheduled += 1,
                            Some(LeavePeriodStatus::Processed) => processed += 1,
                            Some(LeavePeriodStatus::Requested) => requested += 1,
                            Some(LeavePeriodStatus::Rejected) => rejected += 1,
                            None => {}
                        }
                    }
                }
            }

            info!(
                "Status breakdown: Scheduled={}, Processed={}, Requested={}, Rejected={}",
                scheduled, processed, requested, rejected
            );
        }
        Err(xero_rs::error::Error::Forbidden(_)) => {
            info!("Payroll scopes not available, skipping test");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to list leave applications v2: {:?}", e);
            return Err(miette::miette!(
                "Failed to list leave applications v2: {:?}",
                e
            ));
        }
    }

    test_utils::do_cleanup().await;
    info!("Leave applications v2 list test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_list_leave_types() -> miette::Result<()> {
    test_utils::do_setup();
    info!("Starting leave types list test");

    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;

    match client.leave_types().list().await {
        Ok(leave_types) => {
            info!("Found {} leave types", leave_types.len());

            for lt in &leave_types {
                info!(
                    "Leave Type: ID={}, Name={}, Units={:?}, Paid={:?}",
                    lt.leave_type_id, lt.name, lt.type_of_units, lt.is_paid_leave
                );
            }
        }
        Err(xero_rs::error::Error::Forbidden(_)) => {
            info!("Payroll scopes not available, skipping test");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to list leave types: {:?}", e);
            return Err(miette::miette!("Failed to list leave types: {:?}", e));
        }
    }

    test_utils::do_cleanup().await;
    info!("Leave types list test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_get_leave_application() -> miette::Result<()> {
    test_utils::do_setup();
    info!("Starting get leave application by ID test");

    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;

    // First, list to get an ID
    let leave_apps = match client.leave_applications().list_all().await {
        Ok(apps) => apps,
        Err(xero_rs::error::Error::Forbidden(_)) => {
            info!("Payroll scopes not available, skipping test");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to list leave applications: {:?}", e);
            return Err(miette::miette!(
                "Failed to list leave applications: {:?}",
                e
            ));
        }
    };

    if leave_apps.is_empty() {
        info!("No leave applications found, skipping get test");
        return Ok(());
    }

    let first_app = &leave_apps[0];
    info!(
        "Fetching leave application by ID: {}",
        first_app.leave_application_id
    );

    match client
        .leave_applications()
        .get(first_app.leave_application_id)
        .await
    {
        Ok(app) => {
            info!("Successfully retrieved leave application");
            assert_eq!(app.leave_application_id, first_app.leave_application_id);
            assert_eq!(app.employee_id, first_app.employee_id);
            assert_eq!(app.start_date, first_app.start_date);
            assert_eq!(app.end_date, first_app.end_date);
        }
        Err(e) => {
            error!("Failed to get leave application: {:?}", e);
            return Err(miette::miette!("Failed to get leave application: {:?}", e));
        }
    }

    test_utils::do_cleanup().await;
    info!("Get leave application test completed successfully");
    Ok(())
}

#[tokio::test]
async fn test_list_leave_with_employee_filter() -> miette::Result<()> {
    test_utils::do_setup();
    info!("Starting leave applications list with employee filter test");

    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;

    // First, get an employee ID
    let employees = match client.employees().list().await {
        Ok(emps) => emps,
        Err(xero_rs::error::Error::Forbidden(_)) => {
            info!("Payroll scopes not available, skipping test");
            return Ok(());
        }
        Err(e) => {
            error!("Failed to list employees: {:?}", e);
            return Err(miette::miette!("Failed to list employees: {:?}", e));
        }
    };

    if employees.is_empty() {
        info!("No employees found, skipping filter test");
        return Ok(());
    }

    let employee = &employees[0];
    info!("Filtering leave for employee: {}", employee.employee_id);

    let params = ListParameters {
        employee_id: Some(employee.employee_id),
        ..Default::default()
    };

    match client.leave_applications().list(Some(params), None).await {
        Ok(leave_apps) => {
            info!(
                "Found {} leave applications for employee {}",
                leave_apps.len(),
                employee.employee_id
            );

            // Verify all returned leave is for this employee
            for app in &leave_apps {
                assert_eq!(
                    app.employee_id, employee.employee_id,
                    "Leave application should be for the filtered employee"
                );
            }
        }
        Err(e) => {
            error!("Failed to list leave applications with filter: {:?}", e);
            return Err(miette::miette!(
                "Failed to list leave applications with filter: {:?}",
                e
            ));
        }
    }

    test_utils::do_cleanup().await;
    info!("Leave applications filter test completed successfully");
    Ok(())
}
