use chrono::{Datelike, NaiveDate};
use tracing::{debug, error, info};

mod test_utils;

use xero_rs::{
    client::Client,
    entities::timesheet::{CreateTimesheet, Timesheet, TimesheetLine, TimesheetStatus},
    payroll::{
        employee,
        settings::{earnings_rates, pay_calendar},
    },
};

#[tokio::test]
async fn test_timesheet_crud() -> miette::Result<()> {
    test_utils::do_setup();
    info!("Starting timesheet CRUD test");

    let workspace_path = std::env::current_dir().unwrap();
    info!("Current directory: {:?}", workspace_path);

    // Setup

    // Create client with payroll scopes
    let client = test_utils::create_test_client(test_utils::payroll_scopes()).await?;

    let result = match run_test(&client).await {
        Ok(_) => {
            info!("Test completed successfully");
            Ok(())
        }
        Err(e) => {
            error!("Test failed: {:?}", e);
            Err(e)
        }
    };

    // Cleanup
    test_utils::do_cleanup().await;

    result
}

async fn run_test(client: &Client) -> miette::Result<()> {
    // First, get a valid employee ID
    info!("Fetching employees");
    let employees = match employee::list(client).await {
        Ok(employees) => {
            info!("Found {} employees", employees.len());
            employees
        }
        Err(e) => {
            error!("Failed to fetch employees: {:?}", e);
            return Err(miette::miette!("Failed to fetch employees: {:?}", e));
        }
    };

    if employees.is_empty() {
        error!("No employees found in the Xero account");
        return Err(miette::miette!("No employees found in the Xero account"));
    }

    // Use the first employee
    let employee = &employees[0];
    info!("Using employee: ID={}", employee.employee_id);

    // Get the employee's pay calendar ID
    if let Some(payroll_calendar_id) = employee.payroll_calendar_id {
        info!("Employee has payroll calendar ID: {}", payroll_calendar_id);

        // Fetch the pay calendar details
        let pay_calendar = match pay_calendar::get(client, payroll_calendar_id).await {
            Ok(calendar) => {
                info!("Successfully retrieved pay calendar: {}", calendar.name);
                info!(
                    "Pay calendar period: {} to {}",
                    calendar.period_start_date, calendar.period_end_date
                );
                calendar
            }
            Err(e) => {
                error!("Failed to fetch pay calendar: {:?}", e);
                return Err(miette::miette!("Failed to fetch pay calendar: {:?}", e));
            }
        };

        // Use the dates from the pay calendar for our timesheet
        let start_date = pay_calendar.period_start_date;
        let end_date = pay_calendar.period_end_date;

        info!(
            "Using timesheet period from pay calendar: {} to {}",
            start_date, end_date
        );

        // Then, get a valid earnings rate ID
        info!("Fetching earnings rates");
        let earnings_rates = match earnings_rates::list(client).await {
            Ok(rates) => {
                info!("Found {} earnings rates", rates.len());
                rates
            }
            Err(e) => {
                error!("Failed to fetch earnings rates: {:?}", e);
                return Err(miette::miette!("Failed to fetch earnings rates: {:?}", e));
            }
        };

        if earnings_rates.is_empty() {
            error!("No earnings rates found in the Xero account");
            return Err(miette::miette!(
                "No earnings rates found in the Xero account"
            ));
        }

        let earnings_rate = &earnings_rates[0];
        info!(
            "Using earnings rate: ID={}, Name={}",
            earnings_rate.earnings_rate_id, earnings_rate.name
        );

        // Parse dates to calculate number of units
        let start = match NaiveDate::parse_from_str(&start_date, "%Y-%m-%d") {
            Ok(date) => date,
            Err(e) => {
                return Err(miette::miette!(
                    "Failed to parse start date {}: {}",
                    start_date,
                    e
                ))
            }
        };

        let end = match NaiveDate::parse_from_str(&end_date, "%Y-%m-%d") {
            Ok(date) => date,
            Err(e) => {
                return Err(miette::miette!(
                    "Failed to parse end date {}: {}",
                    end_date,
                    e
                ))
            }
        };

        // Calculate the number of days in the period
        let days_in_period = (end - start).num_days() + 1;

        // Create number_of_units array with 8 hours for weekdays, 0 for weekends
        let mut number_of_units = Vec::new();
        let mut total_hours = 0.0;

        for day_offset in 0..days_in_period {
            let date = start + chrono::Duration::days(day_offset);
            let is_weekday = matches!(
                date.weekday(),
                chrono::Weekday::Mon
                | chrono::Weekday::Tue
                | chrono::Weekday::Wed
                | chrono::Weekday::Thu
                | chrono::Weekday::Fri
            );

            // Assign 8 hours for weekdays, 0 for weekends
            let hours = if is_weekday { 8.0 } else { 0.0 };
            number_of_units.push(hours);
            total_hours += hours;
        }

        // Create a test timesheet
        let timesheet_line = TimesheetLine {
            timesheet_line_id: uuid::Uuid::new_v4(),
            date: start_date.clone(),
            earnings_rate_id: earnings_rate.earnings_rate_id,
            number_of_units,
        };

        let create_timesheet = CreateTimesheet {
            employee_id: employee.employee_id,
            start_date,
            end_date,
            status: TimesheetStatus::Draft,
            hours: total_hours,
            timesheet_lines: vec![timesheet_line],
        };

        debug!("Timesheet to create: {:?}", create_timesheet);

        // Create the timesheet
        info!("Creating timesheet...");
        match Timesheet::create(client, &create_timesheet).await {
            Ok(timesheet) => {
                info!("Timesheet created: ID={}", timesheet.timesheet_id);

                // Delete the timesheet
                info!("Deleting timesheet...");
                match Timesheet::delete(client, timesheet.timesheet_id).await {
                    Ok(_) => {
                        info!("Timesheet deleted successfully");
                    }
                    Err(e) => {
                        error!("Failed to delete timesheet: {:?}", e);
                        return Err(miette::miette!("Failed to delete timesheet: {:?}", e));
                    }
                }
            }
            Err(e) => {
                error!("Failed to create timesheet: {:?}", e);
                return Err(miette::miette!("Failed to create timesheet: {:?}", e));
            }
        }

        Ok(())
    } else {
        error!("Employee does not have a payroll calendar ID");
        Err(miette::miette!(
            "Employee does not have a payroll calendar ID"
        ))
    }
}

#[allow(dead_code)]
async fn do_setup() {
    // Any setup code
}

#[allow(dead_code)]
async fn do_cleanup() {
    // Any cleanup code
}
