use tracing::{error, info};
use uuid::Uuid;
use time::macros::date;

mod test_utils;

use xero_rs::{
    client::Client,
    payroll::settings::pay_calendar::{CalendarType, CreatePayCalendar},
};

#[tokio::test]
async fn test_pay_calendar_api() -> miette::Result<()> {
    test_utils::do_setup();
    info!("Starting pay calendar API test");

    let workspace_path = std::env::current_dir().unwrap();
    info!("Current directory: {:?}", workspace_path);

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
    // Test list pay calendars
    info!("Listing pay calendars");
    let pay_calendars = match client.pay_calendars().list().await {
        Ok(calendars) => calendars,
        Err(e) => return Err(miette::miette!("Failed to list pay calendars: {:?}", e)),
    };
    info!("Found {} pay calendars", pay_calendars.len());
    
    if !pay_calendars.is_empty() {
        // Test get pay calendar by ID
        let pay_calendar_id = pay_calendars[0].pay_calendar_id;
        info!("Getting pay calendar with ID: {}", pay_calendar_id);
        let pay_calendar = match client.pay_calendars().get(pay_calendar_id).await {
            Ok(calendar) => calendar,
            Err(e) => return Err(miette::miette!("Failed to get pay calendar: {:?}", e)),
        };
        info!("Found pay calendar: {}", pay_calendar.name);
        
        // Verify the retrieved pay calendar matches the expected
        assert_eq!(pay_calendar.pay_calendar_id, pay_calendar_id);
    } else {
        info!("No existing pay calendars found, skipping get test");
    }
    
    // Test create pay calendar
    info!("Creating new pay calendar");
    let calendar_name = format!("Test Calendar {}", Uuid::new_v4());
    let new_pay_calendar = CreatePayCalendar {
        name: calendar_name.clone(),
        calendar_type: CalendarType::Weekly,
        start_date: date!(2023-01-01),
        payment_date: date!(2023-01-07),
    };
    
    let created_pay_calendar = match client.pay_calendars().create(&new_pay_calendar).await {
        Ok(calendar) => calendar,
        Err(e) => return Err(miette::miette!("Failed to create pay calendar: {:?}", e)),
    };
    info!("Created pay calendar: {}", created_pay_calendar.name);
    
    // Verify the created pay calendar (only check name and calendar_type)
    assert_eq!(created_pay_calendar.name, calendar_name);
    assert_eq!(created_pay_calendar.calendar_type, CalendarType::Weekly);
    
    // Verify we can retrieve the newly created pay calendar
    let retrieved_pay_calendar = match client
        .pay_calendars()
        .get(created_pay_calendar.pay_calendar_id)
        .await {
            Ok(calendar) => calendar,
            Err(e) => return Err(miette::miette!("Failed to retrieve created pay calendar: {:?}", e)),
        };
    info!("Retrieved the newly created pay calendar: {}", retrieved_pay_calendar.name);
    
    // Verify the retrieved pay calendar matches the created one
    assert_eq!(retrieved_pay_calendar.pay_calendar_id, created_pay_calendar.pay_calendar_id);
    assert_eq!(retrieved_pay_calendar.name, created_pay_calendar.name);
    assert_eq!(retrieved_pay_calendar.calendar_type, CalendarType::Weekly);
    
    Ok(())
} 