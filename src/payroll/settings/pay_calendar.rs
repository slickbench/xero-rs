use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::{client::Client, error::Result};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayCalendar {
    #[serde(rename = "PayCalendarID")]
    pub pay_calendar_id: Uuid,
    pub calendar_type: String,
    pub name: String,
    pub period_start_date: String,
    pub period_end_date: String,
    pub payment_date: String,
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayCalendarResponse {
    pub pay_calendars: Vec<PayCalendar>,
}

/// Lists all pay calendars
pub async fn list(client: &Client) -> Result<Vec<PayCalendar>> {
    info!("Listing pay calendars");

    let url = "https://api.xero.com/payroll.xro/1.0/PayrollCalendars";
    debug!("GET URL: {}", url);

    let response: PayCalendarResponse = match client.get(url, &()).await {
        Ok(response) => {
            info!("Pay calendars retrieval successful");
            response
        }
        Err(e) => {
            error!("Error retrieving pay calendars: {:?}", e);
            return Err(e);
        }
    };

    debug!(
        "Response contains {} pay calendars",
        response.pay_calendars.len()
    );
    Ok(response.pay_calendars)
}

/// Gets a pay calendar by ID
/// 
/// # Panics
/// 
/// This function will panic if the response contains pay calendars but the first element cannot be accessed.
pub async fn get(client: &Client, pay_calendar_id: Uuid) -> Result<PayCalendar> {
    info!("Getting pay calendar with ID: {}", pay_calendar_id);

    let url = format!("https://api.xero.com/payroll.xro/1.0/PayrollCalendars/{pay_calendar_id}");
    debug!("GET URL: {}", url);

    let response: PayCalendarResponse = match client.get(&url, &()).await {
        Ok(response) => {
            info!("Pay calendar retrieval successful");
            response
        }
        Err(e) => {
            error!("Error retrieving pay calendar: {:?}", e);
            return Err(e);
        }
    };

    if response.pay_calendars.is_empty() {
        error!("Received empty pay calendars array in response");
        return Err(crate::error::Error::NotFound {
            entity: "PayCalendar".to_string(),
            url,
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some(format!("{response:?}")),
        });
    }

    debug!(
        "Response contains {} pay calendars",
        response.pay_calendars.len()
    );
    Ok(response.pay_calendars.into_iter().next().unwrap())
}
