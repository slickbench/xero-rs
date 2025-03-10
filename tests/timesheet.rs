use chrono::{Datelike, NaiveDate, NaiveDateTime};
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
    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;

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
    let employees = match client.employees().list().await {
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
        let pay_calendar = match client.pay_calendars().get(payroll_calendar_id).await {
            Ok(calendar) => {
                info!("Successfully retrieved pay calendar: {}", calendar.name);
                info!(
                    "Pay calendar period: {} to payment date {}",
                    calendar.start_date, calendar.payment_date
                );
                calendar
            }
            Err(e) => {
                error!("Failed to fetch pay calendar: {:?}", e);
                return Err(miette::miette!("Failed to fetch pay calendar: {:?}", e));
            }
        };

        // Use the dates from the pay calendar for our timesheet
        let start_date = pay_calendar.start_date;
        let end_date = pay_calendar.payment_date;

        info!(
            "Using timesheet period from pay calendar: {} to {}",
            start_date, end_date
        );

        // Then, get a valid earnings rate ID
        info!("Fetching earnings rates");
        let earnings_rates = match client.earnings_rates().list().await {
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

        // Helper function to parse Xero date format
        fn parse_xero_date(xero_date: &str) -> Result<String, &'static str> {
            // Xero date format looks like: /Date(1741996800000+0000)/
            let regex_pattern = r"/Date\((\d+)(?:[-+]\d{4})?\)/";
            let re = regex::Regex::new(regex_pattern).unwrap();
            
            if let Some(captures) = re.captures(xero_date) {
                if let Some(timestamp_match) = captures.get(1) {
                    let timestamp_ms: i64 = timestamp_match
                        .as_str()
                        .parse()
                        .map_err(|_| "Failed to parse timestamp as i64")?;
                    
                    // Convert milliseconds to seconds for chrono
                    let timestamp_sec = timestamp_ms / 1000;
                    
                    // Convert Unix timestamp to NaiveDateTime
                    let datetime = NaiveDateTime::from_timestamp_opt(timestamp_sec, 0)
                        .ok_or("Invalid timestamp")?;
                    
                    // Format the date as YYYY-MM-DD
                    return Ok(datetime.format("%Y-%m-%d").to_string());
                }
            }
            
            Err("Failed to parse Xero date format")
        }

        // Parse dates to calculate number of units
        let parsed_start_date = match parse_xero_date(&start_date) {
            Ok(date) => date,
            Err(e) => {
                return Err(miette::miette!(
                    "Failed to parse start date {}: {}",
                    start_date,
                    e
                ))
            }
        };

        // Parse the start date
        let start = match NaiveDate::parse_from_str(&parsed_start_date, "%Y-%m-%d") {
            Ok(date) => date,
            Err(e) => {
                return Err(miette::miette!(
                    "Failed to parse parsed start date {}: {}",
                    parsed_start_date,
                    e
                ))
            }
        };

        // Calculate end date based on calendar type
        // For a FORTNIGHTLY calendar, add 13 days (2 weeks - 1 day) to the start date
        let end = match pay_calendar.calendar_type.as_str() {
            "FORTNIGHTLY" => start + chrono::Duration::days(13),
            "WEEKLY" => start + chrono::Duration::days(6),
            "MONTHLY" => {
                // For a monthly calendar, go to the end of the month
                let (year, month, _) = (start.year(), start.month(), start.day());
                let days_in_month = if month == 12 {
                    NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
                };
                days_in_month.pred() // Last day of the month
            }
            _ => {
                // Default to two weeks if we don't recognize the calendar type
                start + chrono::Duration::days(13)
            }
        };

        // Format dates for API
        let formatted_start_date = start.format("%Y-%m-%d").to_string();
        let formatted_end_date = end.format("%Y-%m-%d").to_string();
        
        info!(
            "Using timesheet period: {} to {}",
            formatted_start_date, formatted_end_date
        );

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
            timesheet_line_id: Some(uuid::Uuid::new_v4()),
            date: Some(formatted_start_date.clone()),
            earnings_rate_id: earnings_rate.earnings_rate_id,
            number_of_units,
            updated_date_utc: None,
        };

        // Try to find an existing timesheet for this employee
        info!("Checking for existing timesheets...");
        let existing_timesheets = match Timesheet::list(client).await {
            Ok(timesheets) => {
                info!("Found {} existing timesheets", timesheets.len());
                timesheets
            }
            Err(e) => {
                error!("Error listing timesheets: {:?}", e);
                return Err(miette::miette!("Error listing timesheets: {:?}", e));
            }
        };

        // Check if there's a timesheet for our employee
        let employee_timesheet = existing_timesheets.iter().find(|t| t.employee_id == employee.employee_id);

        if let Some(timesheet) = employee_timesheet {
            if timesheet.status == TimesheetStatus::Processed {
                info!("Timesheet is already {:?}, skipping update", timesheet.status);
            } else {
                // Update the existing timesheet
                info!("Updating existing timesheet with ID: {}", timesheet.timesheet_id);
                
                // Clone the existing timesheet and update the fields we want to change
                let mut updated_timesheet = timesheet.clone();
                updated_timesheet.start_date = formatted_start_date;
                updated_timesheet.end_date = formatted_end_date;
                updated_timesheet.hours = total_hours;
                updated_timesheet.status = TimesheetStatus::Draft;
                
                // Replace the timesheet lines
                updated_timesheet.timesheet_lines = vec![timesheet_line];
                
                match Timesheet::update(client, &updated_timesheet).await {
                    Ok(updated) => {
                        info!("Timesheet updated successfully: ID={}", updated.timesheet_id);
                    }
                    Err(e) => {
                        error!("Failed to update timesheet: {:?}", e);
                        return Err(miette::miette!("Failed to update timesheet: {:?}", e));
                    }
                }
            }
        } else {
            // Create a new timesheet
            info!("No existing timesheet found, creating a new one...");
            
            let create_timesheet = CreateTimesheet {
                employee_id: employee.employee_id,
                start_date: formatted_start_date,
                end_date: formatted_end_date,
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

                    // Mark the timesheet as processed instead of deleting it
                    info!("Marking timesheet as processed...");
                    let mut processed_timesheet = timesheet.clone();
                    processed_timesheet.status = TimesheetStatus::Processed;
                    
                    match Timesheet::update(client, &processed_timesheet).await {
                        Ok(processed) => {
                            info!("Timesheet marked as processed successfully: ID={}", processed.timesheet_id);
                        }
                        Err(e) => {
                            error!("Failed to mark timesheet as processed: {:?}", e);
                            return Err(miette::miette!("Failed to mark timesheet as processed: {:?}", e));
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create timesheet: {:?}", e);
                    return Err(miette::miette!("Failed to create timesheet: {:?}", e));
                }
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
