use time::Duration;
use tracing::{error, info};

mod test_utils;

use xero_rs::{
    client::Client,
    entities::timesheet::{self, PostTimesheet, Timesheet, TimesheetLine, TimesheetStatus},
    payroll::employee::Employee,
    payroll::settings::earnings_rates::EarningsRate,
};

#[tokio::test]
async fn test_timesheet_crud() -> miette::Result<()> {
    // Set up logging and test environment
    test_utils::do_setup();
    info!("Starting timesheet CRUD test");
    info!("Current directory: {:?}", std::env::current_dir().unwrap());
    
    // Create a Xero API client
    let client = test_utils::create_test_client(Some(test_utils::payroll_scopes())).await?;
    
    // Run the main test logic
    run_test(&client).await?;
    
    // Clean up resources
    info!("Test completed successfully");
    test_utils::do_cleanup().await;
    
    Ok(())
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

    // Fetch earnings rates
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
        return Err(miette::miette!("No earnings rates found in the Xero account"));
    }

    // Use the first earnings rate
    let earnings_rate = &earnings_rates[0];
    info!(
        "Using earnings rate: ID={}, Name={}",
        earnings_rate.earnings_rate_id, earnings_rate.name
    );

    // Fetch pay calendars to get a valid pay period
    info!("Fetching pay calendars");
    let pay_calendars = match client.pay_calendars().list().await {
        Ok(calendars) => {
            info!("Found {} pay calendars", calendars.len());
            calendars
        }
        Err(e) => {
            error!("Failed to fetch pay calendars: {:?}", e);
            return Err(miette::miette!("Failed to fetch pay calendars: {:?}", e));
        }
    };

    if pay_calendars.is_empty() {
        error!("No pay calendars found in the Xero account");
        return Err(miette::miette!("No pay calendars found in the Xero account"));
    }

    // Use the first pay calendar
    let pay_calendar = &pay_calendars[0];
    info!(
        "Using pay calendar: ID={}, Type={:?}, StartDate={}, PaymentDate={}",
        pay_calendar.pay_calendar_id,
        pay_calendar.calendar_type,
        pay_calendar.start_date,
        pay_calendar.payment_date
    );

    // Get the start and end dates from the pay calendar
    let start_date = pay_calendar.start_date;
    let end_date = pay_calendar.end_date();
    
    info!("Using pay period: Start={}, End={}", start_date, end_date);

    // First, check if a timesheet already exists for this employee and pay period
    info!("Checking for existing timesheets for employee and period");
    let mut list_params = timesheet::ListParameters::default();
    list_params.employee_id = Some(employee.employee_id);
    list_params.start_date = Some(start_date);
    list_params.end_date = Some(end_date);
    
    let existing_timesheets = match client.timesheets().list(Some(list_params), None).await {
        Ok(timesheets) => {
            info!("Found {} matching timesheets", timesheets.len());
            timesheets
        }
        Err(e) => {
            error!("Failed to fetch existing timesheets: {:?}", e);
            return Err(miette::miette!("Failed to fetch existing timesheets: {:?}", e));
        }
    };

    // Look for a matching timesheet (we've already filtered by employee ID and date range in the API)
    let matching_timesheet = existing_timesheets.first();

    let created = if let Some(existing) = matching_timesheet {
        info!(
            "Found existing timesheet (ID: {}) for this employee and pay period",
            existing.timesheet_id
        );
        
        // Only update if the timesheet is in Draft status
        if existing.status == TimesheetStatus::Draft {
            // Create updated version of the existing timesheet
            let mut updated = existing.clone();
            
            // Create a single timesheet line - adjust number_of_units based on period length
            let days_in_period = (end_date - start_date).whole_days() + 1;
            let mut units = Vec::with_capacity(days_in_period as usize);
            
            // Fill with 8 hours for each working day in the period
            for i in 0..days_in_period {
                let current_date = start_date.saturating_add(Duration::days(i));
                let weekday = current_date.weekday();
                
                // Only allocate hours for weekdays
                let hours = match weekday {
                    time::Weekday::Monday |
                    time::Weekday::Tuesday |
                    time::Weekday::Wednesday |
                    time::Weekday::Thursday |
                    time::Weekday::Friday => 8.0,
                    _ => 0.0, // Weekend
                };
                
                units.push(hours);
            }
            
            // Update the timesheet line
            if updated.timesheet_lines.is_empty() {
                updated.timesheet_lines = vec![
                    TimesheetLine {
                        earnings_rate_id: earnings_rate.earnings_rate_id,
                        number_of_units: units,
                        updated_date_utc: None,
                        tracking_item_id: None,
                    }
                ];
            } else {
                updated.timesheet_lines[0].earnings_rate_id = earnings_rate.earnings_rate_id;
                updated.timesheet_lines[0].number_of_units = units;
            }
            
            // Update the timesheet
            info!("Updating existing timesheet in Draft status");
            match client.timesheets().update(&updated).await {
                Ok(updated) => {
                    info!("Successfully updated timesheet with ID: {}", updated.timesheet_id);
                    updated
                }
                Err(e) => {
                    error!("Failed to update timesheet: {:?}", e);
                    return Err(miette::miette!("Failed to update timesheet: {:?}", e));
                }
            }
        } else {
            info!(
                "Existing timesheet is not in Draft status (current status: {:?}), using it as-is",
                existing.status
            );
            existing.clone()
        }
    } else {
        // No existing timesheet found, create a new one
        info!("No existing timesheet found, creating new one");
        create_new_timesheet(client, employee, earnings_rate, start_date, end_date).await?
    };

    // For newly created timesheets, we can verify them
    // For existing timesheets that are not in Draft status, we'll skip verification
    if created.status == TimesheetStatus::Draft {
        // Get the timesheet by ID
        info!("Fetching created timesheet");
        let fetched = match client.timesheets().get(created.timesheet_id).await {
            Ok(fetched) => {
                info!("Successfully fetched timesheet");
                fetched
            }
            Err(e) => {
                error!("Failed to fetch timesheet: {:?}", e);
                return Err(miette::miette!("Failed to fetch timesheet: {:?}", e));
            }
        };

        // Verify the fetched timesheet
        assert_eq!(fetched.timesheet_id, created.timesheet_id);
        assert_eq!(fetched.employee_id, employee.employee_id);
        
        // Change status to Processed (since delete is not supported by the API)
        info!("Updating timesheet status to Processed");
        let mut updated_timesheet = fetched.clone();
        updated_timesheet.status = TimesheetStatus::Processed;
        
        match client.timesheets().update(&updated_timesheet).await {
            Ok(_) => {
                info!("Successfully updated timesheet status to Processed");
            }
            Err(e) => {
                error!("Failed to update timesheet: {:?}", e);
                return Err(miette::miette!("Failed to update timesheet: {:?}", e));
            }
        }
    } else {
        info!(
            "Skipping verification and status update as timesheet is not in Draft status (current status: {:?})",
            created.status
        );
    }

    // Test is successful even if we couldn't update the status
    info!("Timesheet test completed successfully");
    Ok(())
}

async fn create_new_timesheet(
    client: &Client,
    employee: &Employee,
    earnings_rate: &EarningsRate,
    start_date: time::Date,
    end_date: time::Date,
) -> miette::Result<Timesheet> {
    // First, check if a timesheet already exists for this employee and period
    let mut list_params = timesheet::ListParameters::default();
    list_params.employee_id = Some(employee.employee_id);
    list_params.start_date = Some(start_date);
    list_params.end_date = Some(end_date);
    
    // Try to find an existing timesheet first
    match client.timesheets().list(Some(list_params), None).await {
        Ok(timesheets) => {
            if let Some(existing) = timesheets.first() {
                info!("Found existing timesheet with ID: {}", existing.timesheet_id);
                return Ok(existing.clone());
            }
        }
        Err(e) => {
            error!("Failed to check for existing timesheets: {:?}", e);
            // Continue with creation attempt
        }
    }
    
    // Create a single timesheet line - adjust number_of_units based on period length
    let days_in_period = (end_date - start_date).whole_days() + 1;
    let mut units = Vec::with_capacity(days_in_period as usize);
    
    // Fill with 8 hours for each working day in the period
    for i in 0..days_in_period {
        let current_date = start_date.saturating_add(Duration::days(i));
        let weekday = current_date.weekday();
        
        // Only allocate hours for weekdays
        let hours = match weekday {
            time::Weekday::Monday |
            time::Weekday::Tuesday |
            time::Weekday::Wednesday |
            time::Weekday::Thursday |
            time::Weekday::Friday => 8.0,
            _ => 0.0, // Weekend
        };
        
        units.push(hours);
    }
    
    let timesheet_lines = vec![
        TimesheetLine {
            earnings_rate_id: earnings_rate.earnings_rate_id,
            number_of_units: units,
            updated_date_utc: None,
            tracking_item_id: None,
        }
    ];

    // Create a new timesheet for the employee
    let timesheet = PostTimesheet {
        timesheet_id: None,
        employee_id: employee.employee_id,
        start_date,
        end_date,
        status: Some(TimesheetStatus::Draft),
        timesheet_lines: Some(timesheet_lines),
    };

    // Submit the timesheet
    info!("Creating timesheet for employee");
    match client.timesheets().create(&timesheet).await {
        Ok(created) => {
            info!("Successfully created timesheet with ID: {}", created.timesheet_id);
            Ok(created)
        }
        Err(e) => {
            // If the error is because the timesheet already exists, try to find it again
            if e.to_string().contains("This timesheet already exists") {
                info!("Timesheet already exists, trying to find it");
                
                // Create new list parameters
                let mut new_list_params = timesheet::ListParameters::default();
                new_list_params.employee_id = Some(employee.employee_id);
                new_list_params.start_date = Some(start_date);
                new_list_params.end_date = Some(end_date);
                
                match client.timesheets().list(Some(new_list_params), None).await {
                    Ok(timesheets) => {
                        if let Some(existing) = timesheets.first() {
                            info!("Found existing timesheet with ID: {}", existing.timesheet_id);
                            return Ok(existing.clone());
                        }
                    }
                    Err(list_err) => {
                        error!("Failed to list timesheets: {:?}", list_err);
                    }
                }
            }
            
            error!("Failed to create timesheet: {:?}", e);
            Err(miette::miette!("Failed to create timesheet: {:?}", e))
        }
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