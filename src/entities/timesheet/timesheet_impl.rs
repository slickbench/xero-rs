use serde::{Deserialize, Serialize};
use tracing::{debug, error, info};
use uuid::Uuid;
use time::Date;

use super::{TimesheetLine, TimesheetStatus};
use crate::{
    error::Result,
    utils::date_format::{xero_date_format, xero_date_format_option},
};

/// Parameters for filtering timesheet list results
#[derive(Debug, Serialize, Default)]
pub struct ListParameters {
    /// The employee ID to filter by
    #[serde(rename = "EmployeeId", skip_serializing_if = "Option::is_none")]
    pub employee_id: Option<Uuid>,
    
    /// Filter by status (e.g., "DRAFT", "APPROVED", "PROCESSED")
    #[serde(rename = "Status", skip_serializing_if = "Option::is_none")]
    pub status: Option<TimesheetStatus>,
    
    /// Filter by start date (timesheets that start on or after this date)
    #[serde(rename = "StartDate", with = "xero_date_format_option", skip_serializing_if = "Option::is_none")]
    pub start_date: Option<Date>,
    
    /// Filter by end date (timesheets that end on or before this date)
    #[serde(rename = "EndDate", with = "xero_date_format_option", skip_serializing_if = "Option::is_none")]
    pub end_date: Option<Date>,
    
    /// Page number for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i32>,
    
    /// Filter by any field using Xero's WHERE syntax
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub where_filter: Option<String>,
    
    /// Order results by a specific field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PostTimesheet {
    #[serde(rename = "TimesheetID", skip_serializing_if = "Option::is_none")]
    pub timesheet_id: Option<Uuid>,
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,
    #[serde(with = "xero_date_format")]
    pub start_date: Date,
    #[serde(with = "xero_date_format")]
    pub end_date: Date,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TimesheetStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timesheet_lines: Option<Vec<TimesheetLine>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Timesheet {
    #[serde(rename = "TimesheetID")]
    pub timesheet_id: Uuid,
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,
    #[serde(with = "xero_date_format")]
    pub start_date: Date,
    #[serde(with = "xero_date_format")]
    pub end_date: Date,
    pub status: TimesheetStatus,
    pub hours: f64,
    pub timesheet_lines: Vec<TimesheetLine>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TimesheetRequest {
    pub timesheets: Vec<Timesheet>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TimesheetResponse {
    pub timesheets: Vec<Timesheet>,
}

impl Timesheet {
    /// Creates a new timesheet
    /// 
    /// # Panics
    /// 
    /// This function will panic if the response contains timesheets but the first element cannot be accessed.
    pub async fn post(
        client: &crate::client::Client,
        timesheet: &PostTimesheet,
    ) -> Result<Timesheet> {
        info!("Creating timesheet");
        debug!("Timesheet data: {:?}", timesheet);

        let request = vec![timesheet.clone()];

        debug!("Sending request to create timesheet");
        let url = "https://api.xero.com/payroll.xro/1.0/Timesheets";
        debug!("POST URL: {}", url);

        let response: TimesheetResponse = match client.post(url, &request).await {
            Ok(response) => {
                info!("Timesheet creation successful");
                response
            }
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
    /// 
    /// # Panics
    /// 
    /// This function will panic if the response contains timesheets but the first element cannot be accessed.
    pub async fn get(client: &crate::client::Client, timesheet_id: Uuid) -> Result<Timesheet> {
        info!("Getting timesheet with ID: {}", timesheet_id);

        let url = format!("https://api.xero.com/payroll.xro/1.0/Timesheets/{timesheet_id}");
        debug!("GET URL: {}", url);

        let response: TimesheetResponse = match client.get(&url, &()).await {
            Ok(response) => {
                info!("Timesheet retrieval successful");
                response
            }
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

    /// Gets all timesheets with optional filtering
    ///
    /// # Parameters
    ///
    /// * `client` - The Xero client
    /// * `parameters` - Optional filter parameters, including `employee_id`, status, date range, page, where, order
    /// * `modified_after` - Optional ISO8601 timestamp to filter by modification date
    pub async fn list(
        client: &crate::client::Client, 
        parameters: Option<&ListParameters>,
        modified_after: Option<String>
    ) -> Result<Vec<Timesheet>> {
        info!("Listing timesheets with filters: {:?}", parameters);

        let url = "https://api.xero.com/payroll.xro/1.0/Timesheets";
        debug!("GET URL: {}", url);

        // Build the request with parameters and headers
        let mut request = client.build_request(reqwest::Method::GET, url);
        
        // Add If-Modified-Since header if provided
        if let Some(date) = modified_after {
            request = request.header("If-Modified-Since", date);
        }

        // Add query parameters
        if let Some(params) = parameters {
            request = request.query(params);
        }

        // Send the request
        let response = request.send().await?;
        let status = response.status();
        
        if !status.is_success() {
            error!("Error listing timesheets: HTTP status {}", status);
            return Err(crate::error::Error::API(
                serde_json::from_str(&response.text().await?)?,
            ));
        }

        // Parse the response
        let response: TimesheetResponse = response.json().await?;

        debug!("Response contains {} timesheets", response.timesheets.len());
        Ok(response.timesheets)
    }

    /// Updates a timesheet
    /// 
    /// # Panics
    /// 
    /// This function will panic if the response contains timesheets but the first element cannot be accessed.
    pub async fn update(
        client: &crate::client::Client,
        timesheet: &Timesheet,
    ) -> Result<Timesheet> {
        info!("Updating timesheet with ID: {}", timesheet.timesheet_id);
        debug!("Updated timesheet data: {:?}", timesheet);

        let request = vec![timesheet.clone()];

        let url = format!(
            "https://api.xero.com/payroll.xro/1.0/Timesheets/{}",
            timesheet.timesheet_id
        );
        debug!("POST URL: {}", url);

        let response: TimesheetResponse = match client.post(&url, &request).await {
            Ok(response) => {
                info!("Timesheet update successful");
                response
            }
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

    // Note: Timesheets cannot be deleted via the Xero API. Instead, update their status to "Processed".
    // The delete method has been removed as it is not supported by the Xero API.
}
