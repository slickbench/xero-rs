use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::{debug, error, info};

use crate::error::Result;
use super::{TimesheetLine, TimesheetStatus};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreateTimesheet {
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,
    pub start_date: String,
    pub end_date: String,
    pub status: TimesheetStatus,
    pub hours: f64,
    pub timesheet_lines: Vec<TimesheetLine>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Timesheet {
    #[serde(rename = "TimesheetID")]
    pub timesheet_id: Uuid,
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,
    pub start_date: String,
    pub end_date: String,
    pub status: TimesheetStatus,
    pub hours: f64,
    pub timesheet_lines: Vec<TimesheetLine>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TimesheetRequest {
    pub timesheets: Vec<Timesheet>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TimesheetResponse {
    pub timesheets: Vec<Timesheet>,
}

impl Timesheet {
    /// Creates a new timesheet
    pub async fn create(client: &crate::client::Client, timesheet: &CreateTimesheet) -> Result<Timesheet> {
        info!("Creating timesheet");
        debug!("Timesheet data: {:?}", timesheet);
        
        let request = vec![timesheet.clone()];
        
        debug!("Sending request to create timesheet");
        let url = "https://api.xero.com/payroll.xro/1.0/Timesheets";
        debug!("POST URL: {}", url);

        let response: TimesheetResponse = match client
            .post(url, &request)
            .await {
                Ok(response) => {
                    info!("Timesheet creation successful");
                    response
                },
                Err(e) => {
                    error!("Error creating timesheet: {:?}", e);
                    return Err(e);
                }
            };

        if response.timesheets.is_empty() {
            error!("Received empty timesheets array in response");
            return Err(crate::error::Error::NotFound {
                entity: "Timesheet".to_string(),
                url: url.to_string(),
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
            });
        }

        debug!("Response contains {} timesheets", response.timesheets.len());
        Ok(response.timesheets.into_iter().next().unwrap())
    }

    /// Gets a timesheet by ID
    pub async fn get(client: &crate::client::Client, timesheet_id: Uuid) -> Result<Timesheet> {
        info!("Getting timesheet with ID: {}", timesheet_id);
        
        let url = format!("https://api.xero.com/payroll.xro/1.0/Timesheets/{timesheet_id}");
        debug!("GET URL: {}", url);
        
        let response: TimesheetResponse = match client
            .get(&url, &())
            .await {
                Ok(response) => {
                    info!("Timesheet retrieval successful");
                    response
                },
                Err(e) => {
                    error!("Error retrieving timesheet: {:?}", e);
                    return Err(e);
                }
            };

        if response.timesheets.is_empty() {
            error!("Received empty timesheets array in response");
            return Err(crate::error::Error::NotFound {
                entity: "Timesheet".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
            });
        }

        debug!("Response contains {} timesheets", response.timesheets.len());
        Ok(response.timesheets.into_iter().next().unwrap())
    }

    /// Gets all timesheets
    pub async fn list(client: &crate::client::Client) -> Result<Vec<Timesheet>> {
        info!("Listing all timesheets");
        
        let url = "https://api.xero.com/payroll.xro/1.0/Timesheets";
        debug!("GET URL: {}", url);
        
        let response: TimesheetResponse = match client
            .get(url, &())
            .await {
                Ok(response) => {
                    info!("Timesheet list retrieval successful");
                    response
                },
                Err(e) => {
                    error!("Error listing timesheets: {:?}", e);
                    return Err(e);
                }
            };

        debug!("Response contains {} timesheets", response.timesheets.len());
        Ok(response.timesheets)
    }

    /// Updates a timesheet
    pub async fn update(client: &crate::client::Client, timesheet: &Timesheet) -> Result<Timesheet> {
        info!("Updating timesheet with ID: {}", timesheet.timesheet_id);
        debug!("Updated timesheet data: {:?}", timesheet);
        
        let request = vec![timesheet.clone()];

        let url = format!(
            "https://api.xero.com/payroll.xro/1.0/Timesheets/{}",
            timesheet.timesheet_id
        );
        debug!("POST URL: {}", url);

        let response: TimesheetResponse = match client
            .post(&url, &request)
            .await {
                Ok(response) => {
                    info!("Timesheet update successful");
                    response
                },
                Err(e) => {
                    error!("Error updating timesheet: {:?}", e);
                    return Err(e);
                }
            };

        if response.timesheets.is_empty() {
            error!("Received empty timesheets array in response");
            return Err(crate::error::Error::NotFound {
                entity: "Timesheet".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
            });
        }

        debug!("Response contains {} timesheets", response.timesheets.len());
        Ok(response.timesheets.into_iter().next().unwrap())
    }

    /// Deletes a timesheet
    pub async fn delete(client: &crate::client::Client, timesheet_id: Uuid) -> Result<()> {
        info!("Deleting timesheet with ID: {}", timesheet_id);
        
        let url = format!(
            "https://api.xero.com/payroll.xro/1.0/Timesheets/{timesheet_id}"
        );
        debug!("DELETE URL: {}", url);
        
        match client.delete(&url).await {
            Ok(()) => {
                info!("Timesheet deletion successful");
                Ok(())
            },
            Err(e) => {
                error!("Error deleting timesheet: {:?}", e);
                Err(e)
            }
        }
    }
} 