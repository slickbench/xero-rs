//! Leave Applications API for Xero Payroll AU
//!
//! This module provides functionality for managing leave applications in Xero Payroll.
//! Leave applications represent requests for employee leave (annual leave, sick leave, etc.).
//!
//! # Example
//!
//! ```no_run
//! use xero_rs::{Client, KeyPair};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let key_pair = KeyPair::from_env();
//! let client = Client::from_client_credentials(key_pair, None).await?;
//!
//! // List all approved leave applications
//! let leave_apps = client.leave_applications().list(None, None).await?;
//!
//! // List ALL leave (including pending/rejected) using v2 endpoint
//! let all_leave = client.leave_applications().list_v2(None, None).await?;
//!
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use time::{Date, OffsetDateTime};
use tracing::{debug, error, info};
use tracing_error::SpanTrace;
use uuid::Uuid;

use crate::{
    error::Result,
    utils::date_format::{xero_date_format, xero_date_format_option, xero_datetime_format_option},
};

/// Base endpoint for leave applications (v1 - returns only approved leave)
pub const ENDPOINT: &str = "https://api.xero.com/payroll.xro/1.0/LeaveApplications";

/// V2 endpoint that returns leave with all statuses (REQUESTED, REJECTED, PROCESSED, SCHEDULED)
pub const ENDPOINT_V2: &str = "https://api.xero.com/payroll.xro/1.0/LeaveApplications/v2";

/// Status of a leave period within a leave application
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LeavePeriodStatus {
    /// Leave is scheduled (default status)
    Scheduled,
    /// Leave has been processed in a pay run
    Processed,
    /// Leave is awaiting approval (v2 endpoint only)
    Requested,
    /// Leave application was rejected (v2 endpoint only)
    Rejected,
}

impl Default for LeavePeriodStatus {
    fn default() -> Self {
        Self::Scheduled
    }
}

/// How the leave will be paid out
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum PayOutType {
    /// Standard leave payment
    #[serde(rename = "DEFAULT")]
    Default,
    /// Leave cashed out instead of taken
    #[serde(rename = "CASHED_OUT")]
    CashedOut,
}

impl Default for PayOutType {
    fn default() -> Self {
        Self::Default
    }
}

/// A period of leave within a leave application
///
/// Leave applications can span multiple pay periods, with each period
/// having its own number of units and status.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LeavePeriod {
    /// Number of units (hours or days) for this period
    #[serde(default)]
    pub number_of_units: Option<f64>,

    /// Start date of the pay period
    #[serde(
        default,
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub pay_period_start_date: Option<Date>,

    /// End date of the pay period
    #[serde(
        default,
        with = "xero_date_format_option",
        skip_serializing_if = "Option::is_none"
    )]
    pub pay_period_end_date: Option<Date>,

    /// Status of this leave period
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leave_period_status: Option<LeavePeriodStatus>,
}

/// Parameters for filtering leave application list results
#[derive(Debug, Default)]
pub struct ListParameters {
    /// Filter by employee ID
    pub employee_id: Option<Uuid>,

    /// Filter by start date (leave applications that start on or after this date)
    pub start_date: Option<Date>,

    /// Filter by end date (leave applications that end on or before this date)
    pub end_date: Option<Date>,

    /// Filter by any field using Xero's WHERE syntax (e.g., "Status==\"ACTIVE\"")
    pub where_filter: Option<String>,

    /// Order results by a specific field
    pub order: Option<String>,

    /// Page number for pagination
    pub page: Option<i32>,
}

impl ListParameters {
    /// Build the where clause for the API query
    fn build_where_clause(&self) -> Option<String> {
        let mut clauses = Vec::new();

        if let Some(employee_id) = &self.employee_id {
            clauses.push(format!("EmployeeID==Guid(\"{}\")", employee_id));
        }

        if let Some(filter) = &self.where_filter {
            clauses.push(filter.clone());
        }

        if clauses.is_empty() {
            None
        } else {
            Some(clauses.join(" AND "))
        }
    }

    /// Convert to query parameters for the API request
    fn to_query_params(&self) -> Vec<(&str, String)> {
        let mut params = Vec::new();

        if let Some(where_clause) = self.build_where_clause() {
            params.push(("where", where_clause));
        }

        if let Some(order) = &self.order {
            params.push(("order", order.clone()));
        }

        if let Some(page) = self.page {
            params.push(("page", page.to_string()));
        }

        params
    }
}

/// Request structure for creating a new leave application
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PostLeaveApplication {
    /// Employee ID (required)
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,

    /// Leave type ID (required)
    #[serde(rename = "LeaveTypeID")]
    pub leave_type_id: Uuid,

    /// Title for the leave application
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Start date of leave (required)
    #[serde(with = "xero_date_format")]
    pub start_date: Date,

    /// End date of leave (required)
    #[serde(with = "xero_date_format")]
    pub end_date: Date,

    /// Description of the leave
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// How the leave will be paid out
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pay_out_type: Option<PayOutType>,

    /// Leave periods (optional - Xero will calculate if not provided)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leave_periods: Option<Vec<LeavePeriod>>,
}

/// A leave application in Xero Payroll
///
/// Represents a request for employee leave, including the leave type,
/// dates, and status of each leave period.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LeaveApplication {
    /// Unique identifier for the leave application
    #[serde(rename = "LeaveApplicationID")]
    pub leave_application_id: Uuid,

    /// Employee who the leave is for
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,

    /// Type of leave being taken
    #[serde(rename = "LeaveTypeID")]
    pub leave_type_id: Uuid,

    /// Title of the leave application
    #[serde(default)]
    pub title: Option<String>,

    /// Start date of the leave
    #[serde(with = "xero_date_format")]
    pub start_date: Date,

    /// End date of the leave
    #[serde(with = "xero_date_format")]
    pub end_date: Date,

    /// Description of the leave
    #[serde(default)]
    pub description: Option<String>,

    /// How the leave is being paid out
    #[serde(default)]
    pub pay_out_type: Option<PayOutType>,

    /// Leave periods breaking down the leave by pay period
    #[serde(default)]
    pub leave_periods: Option<Vec<LeavePeriod>>,

    /// Last updated timestamp
    #[serde(
        default,
        rename = "UpdatedDateUTC",
        with = "xero_datetime_format_option"
    )]
    pub updated_date_utc: Option<OffsetDateTime>,
}

/// Response wrapper for leave application API calls
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LeaveApplicationResponse {
    pub leave_applications: Vec<LeaveApplication>,
}

impl LeaveApplication {
    /// List approved leave applications (v1 endpoint)
    ///
    /// This endpoint only returns leave applications that have been approved.
    /// Use `list_v2` to get all leave including pending and rejected.
    ///
    /// # Parameters
    ///
    /// * `client` - The Xero client
    /// * `parameters` - Optional filter parameters
    /// * `modified_after` - Optional ISO8601 timestamp to filter by modification date
    pub async fn list(
        client: &crate::client::Client,
        parameters: Option<&ListParameters>,
        modified_after: Option<String>,
    ) -> Result<Vec<LeaveApplication>> {
        info!("Listing approved leave applications");
        Self::list_internal(client, ENDPOINT, parameters, modified_after).await
    }

    /// List all leave applications including pending/rejected (v2 endpoint)
    ///
    /// This endpoint returns leave with all statuses: SCHEDULED, PROCESSED,
    /// REQUESTED (awaiting approval), and REJECTED.
    ///
    /// # Parameters
    ///
    /// * `client` - The Xero client
    /// * `parameters` - Optional filter parameters
    /// * `modified_after` - Optional ISO8601 timestamp to filter by modification date
    pub async fn list_v2(
        client: &crate::client::Client,
        parameters: Option<&ListParameters>,
        modified_after: Option<String>,
    ) -> Result<Vec<LeaveApplication>> {
        info!("Listing all leave applications (v2 - includes pending/rejected)");
        Self::list_internal(client, ENDPOINT_V2, parameters, modified_after).await
    }

    /// Internal list implementation shared by v1 and v2 endpoints
    async fn list_internal(
        client: &crate::client::Client,
        url: &str,
        parameters: Option<&ListParameters>,
        modified_after: Option<String>,
    ) -> Result<Vec<LeaveApplication>> {
        debug!("GET URL: {}", url);

        let mut request = client.build_request(reqwest::Method::GET, url).await;

        if let Some(date) = modified_after {
            request = request.header("If-Modified-Since", date);
        }

        if let Some(params) = parameters {
            for (key, value) in params.to_query_params() {
                request = request.query(&[(key, value)]);
            }
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            error!("Error listing leave applications: HTTP status {}", status);
            let text = response.text().await?;

            // Handle 403 Forbidden explicitly
            if status == reqwest::StatusCode::FORBIDDEN {
                // Try to deserialize as ForbiddenResponse, or create a generic API error
                if let Ok(forbidden) =
                    serde_json::from_str::<crate::error::ForbiddenResponse>(&text)
                {
                    return Err(crate::error::Error::Forbidden(Box::new(forbidden)));
                }
                // Fall back to generic API error if can't parse as ForbiddenResponse
                return Err(crate::error::Error::API {
                    response: crate::error::Response {
                        error_number: Some(403),
                        status: Some(403),
                        title: Some("Forbidden".to_string()),
                        message: Some("Forbidden - check payroll scopes".to_string()),
                        detail: Some(text),
                        instance: None,
                        error: crate::error::ErrorType::Other("Forbidden".to_string()),
                    },
                    span_trace: SpanTrace::capture(),
                });
            }

            return Err(crate::error::Error::API {
                response: serde_json::from_str(&text)?,
                span_trace: SpanTrace::capture(),
            });
        }

        let response: LeaveApplicationResponse = response.json().await?;

        debug!(
            "Response contains {} leave applications",
            response.leave_applications.len()
        );
        Ok(response.leave_applications)
    }

    /// Get a single leave application by ID
    pub async fn get(
        client: &crate::client::Client,
        leave_application_id: Uuid,
    ) -> Result<LeaveApplication> {
        info!(
            "Getting leave application with ID: {}",
            leave_application_id
        );

        let url = format!("{ENDPOINT}/{leave_application_id}");
        debug!("GET URL: {}", url);

        let response: LeaveApplicationResponse = match client.get(&url, &()).await {
            Ok(response) => {
                info!("Leave application retrieval successful");
                response
            }
            Err(e) => {
                error!("Error retrieving leave application: {:?}", e);
                return Err(e);
            }
        };

        if response.leave_applications.is_empty() {
            error!("Received empty leave applications array in response");
            return Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!(
                    "Leave application with ID {leave_application_id} not found"
                )),
                span_trace: SpanTrace::capture(),
            });
        }

        debug!(
            "Response contains {} leave applications",
            response.leave_applications.len()
        );
        Ok(response.leave_applications.into_iter().next().unwrap())
    }

    /// Create a new leave application
    pub async fn post(
        client: &crate::client::Client,
        leave_application: &PostLeaveApplication,
    ) -> Result<LeaveApplication> {
        info!("Creating leave application");
        debug!("Leave application data: {:?}", leave_application);

        let request = vec![leave_application.clone()];

        debug!("Sending request to create leave application");
        debug!("POST URL: {}", ENDPOINT);

        let response: LeaveApplicationResponse = match client.post(ENDPOINT, &request).await {
            Ok(response) => {
                info!("Leave application creation successful");
                response
            }
            Err(e) => {
                error!("Error creating leave application: {:?}", e);
                return Err(e);
            }
        };

        if response.leave_applications.is_empty() {
            error!("Received empty leave applications array in response");
            return Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url: ENDPOINT.to_string(),
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
                span_trace: SpanTrace::capture(),
            });
        }

        debug!(
            "Response contains {} leave applications",
            response.leave_applications.len()
        );
        Ok(response.leave_applications.into_iter().next().unwrap())
    }

    /// Update an existing leave application
    pub async fn update(
        client: &crate::client::Client,
        leave_application: &LeaveApplication,
    ) -> Result<LeaveApplication> {
        info!(
            "Updating leave application with ID: {}",
            leave_application.leave_application_id
        );
        debug!("Updated leave application data: {:?}", leave_application);

        let request = vec![leave_application.clone()];

        let url = format!("{ENDPOINT}/{}", leave_application.leave_application_id);
        debug!("POST URL: {}", url);

        let response: LeaveApplicationResponse = match client.post(&url, &request).await {
            Ok(response) => {
                info!("Leave application update successful");
                response
            }
            Err(e) => {
                error!("Error updating leave application: {:?}", e);
                return Err(e);
            }
        };

        if response.leave_applications.is_empty() {
            error!("Received empty leave applications array in response");
            return Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
                span_trace: SpanTrace::capture(),
            });
        }

        debug!(
            "Response contains {} leave applications",
            response.leave_applications.len()
        );
        Ok(response.leave_applications.into_iter().next().unwrap())
    }

    /// Approve a leave application that is in REQUESTED status
    ///
    /// This changes the leave status from REQUESTED to SCHEDULED.
    pub async fn approve(
        client: &crate::client::Client,
        leave_application_id: Uuid,
    ) -> Result<LeaveApplication> {
        info!(
            "Approving leave application with ID: {}",
            leave_application_id
        );

        let url = format!("{ENDPOINT}/{leave_application_id}/approve");
        debug!("POST URL: {}", url);

        // Empty body for approve endpoint
        let response: LeaveApplicationResponse = match client.post(&url, &()).await {
            Ok(response) => {
                info!("Leave application approval successful");
                response
            }
            Err(e) => {
                error!("Error approving leave application: {:?}", e);
                return Err(e);
            }
        };

        if response.leave_applications.is_empty() {
            error!("Received empty leave applications array in response");
            return Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
                span_trace: SpanTrace::capture(),
            });
        }

        Ok(response.leave_applications.into_iter().next().unwrap())
    }

    /// Reject a leave application that is in REQUESTED status
    ///
    /// This changes the leave status from REQUESTED to REJECTED.
    pub async fn reject(
        client: &crate::client::Client,
        leave_application_id: Uuid,
    ) -> Result<LeaveApplication> {
        info!(
            "Rejecting leave application with ID: {}",
            leave_application_id
        );

        let url = format!("{ENDPOINT}/{leave_application_id}/reject");
        debug!("POST URL: {}", url);

        // Empty body for reject endpoint
        let response: LeaveApplicationResponse = match client.post(&url, &()).await {
            Ok(response) => {
                info!("Leave application rejection successful");
                response
            }
            Err(e) => {
                error!("Error rejecting leave application: {:?}", e);
                return Err(e);
            }
        };

        if response.leave_applications.is_empty() {
            error!("Received empty leave applications array in response");
            return Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
                span_trace: SpanTrace::capture(),
            });
        }

        Ok(response.leave_applications.into_iter().next().unwrap())
    }
}
