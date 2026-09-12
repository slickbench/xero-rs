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

use std::collections::HashSet;

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
#[derive(Default)]
pub enum LeavePeriodStatus {
    /// Leave is scheduled (default status)
    #[default]
    Scheduled,
    /// Leave has been processed in a pay run
    Processed,
    /// Leave is awaiting approval (v2 endpoint only)
    Requested,
    /// Leave application was rejected (v2 endpoint only)
    Rejected,
}

/// How the leave will be paid out
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub enum PayOutType {
    /// Standard leave payment
    #[serde(rename = "DEFAULT")]
    #[default]
    Default,
    /// Leave cashed out instead of taken
    #[serde(rename = "CASHED_OUT")]
    CashedOut,
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
#[derive(Clone, Debug, Default)]
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
            clauses.push(format!("EmployeeID==Guid(\"{employee_id}\")"));
        }

        for (field, comparison, date) in [
            ("StartDate", ">=", self.start_date),
            ("EndDate", "<=", self.end_date),
        ] {
            if let Some(date) = date {
                clauses.push(format!(
                    "{field}{comparison}DateTime({},{},{})",
                    date.year(),
                    date.month() as u8,
                    date.day()
                ));
            }
        }
        if let Some(filter) = &self.where_filter {
            clauses.push(format!("({filter})"));
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
        if parameters.and_then(|p| p.page).is_some_and(|page| page < 1) {
            return Err(crate::Error::InvalidParameter(
                "page must be positive".into(),
            ));
        }
        let modified_since = modified_after
            .as_deref()
            .map(crate::utils::date_format::parse_dotnet_datetime)
            .transpose()
            .map_err(|_| {
                crate::Error::InvalidParameter("modified_after must be an ISO8601 timestamp".into())
            })?;
        let query = parameters
            .map(ListParameters::to_query_params)
            .unwrap_or_default();
        let response: LeaveApplicationResponse = client
            .get_with_modified_since(url, &query, modified_since)
            .await?;
        Ok(response.leave_applications)
    }

    /// Retrieve a complete, unfiltered snapshot of all V2 applications.
    pub async fn list_all_v2(client: &crate::Client) -> Result<Vec<LeaveApplication>> {
        Self::list_all_from(client, ENDPOINT_V2).await
    }

    async fn list_all_from(client: &crate::Client, url: &str) -> Result<Vec<LeaveApplication>> {
        let mut applications = Vec::new();
        let mut seen = HashSet::new();
        let mut page = 1_i32;
        loop {
            let parameters = ListParameters {
                page: Some(page),
                order: Some("LeaveApplicationID ASC".into()),
                ..Default::default()
            };
            let batch = Self::list_internal(client, url, Some(&parameters), None).await?;
            if batch.is_empty() {
                return Ok(applications);
            }
            for application in batch {
                if !seen.insert(application.leave_application_id) {
                    return Err(crate::Error::InvalidParameter(
                        "leave pagination repeated an application; retry the snapshot".into(),
                    ));
                }
                applications.push(application);
            }
            page = page.checked_add(1).ok_or_else(|| {
                crate::Error::InvalidParameter("leave page limit exceeded".into())
            })?;
        }
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

        if let Some(value) = response.leave_applications.first() {
            debug!(
                "Response contains {} leave applications",
                response.leave_applications.len()
            );
            Ok(value.clone())
        } else {
            error!("Received empty leave applications array in response");
            Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!(
                    "Leave application with ID {leave_application_id} not found"
                )),
                span_trace: SpanTrace::capture(),
            })
        }
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

        if let Some(value) = response.leave_applications.first() {
            debug!(
                "Response contains {} leave applications",
                response.leave_applications.len()
            );
            Ok(value.clone())
        } else {
            error!("Received empty leave applications array in response");
            Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url: ENDPOINT.to_string(),
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
                span_trace: SpanTrace::capture(),
            })
        }
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

        if let Some(value) = response.leave_applications.first() {
            debug!(
                "Response contains {} leave applications",
                response.leave_applications.len()
            );
            Ok(value.clone())
        } else {
            error!("Received empty leave applications array in response");
            Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
                span_trace: SpanTrace::capture(),
            })
        }
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

        if let Some(value) = response.leave_applications.first() {
            Ok(value.clone())
        } else {
            error!("Received empty leave applications array in response");
            Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
                span_trace: SpanTrace::capture(),
            })
        }
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

        if let Some(value) = response.leave_applications.first() {
            Ok(value.clone())
        } else {
            error!("Received empty leave applications array in response");
            Err(crate::error::Error::NotFound {
                entity: "LeaveApplication".to_string(),
                url,
                status_code: reqwest::StatusCode::NOT_FOUND,
                response_body: Some(format!("{response:?}")),
                span_trace: SpanTrace::capture(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use time::macros::date;
    use warp::Filter;

    fn application(id: u128) -> serde_json::Value {
        json!({
            "LeaveApplicationID": Uuid::from_u128(id),
            "EmployeeID": Uuid::nil(), "LeaveTypeID": Uuid::nil(),
            "StartDate": "/Date(1791072000000+0000)/", "EndDate": "2026-10-06",
            "LeavePeriods": [{"LeavePeriodStatus": "REQUESTED", "NumberOfUnits": 7.6}]
        })
    }

    #[test]
    fn date_filters_are_encoded_and_custom_disjunction_is_grouped() {
        let parameters = ListParameters {
            employee_id: Some(Uuid::nil()),
            start_date: Some(date!(2026 - 09 - 10)),
            end_date: Some(date!(2026 - 10 - 01)),
            where_filter: Some("A==1 OR B==2".into()),
            page: Some(2),
            ..Default::default()
        };
        let query = parameters.to_query_params();
        assert_eq!(
            query[0],
            (
                "where",
                format!(
                    "EmployeeID==Guid(\"{}\") AND StartDate>=DateTime(2026,9,10) AND EndDate<=DateTime(2026,10,1) AND (A==1 OR B==2)",
                    Uuid::nil()
                )
            )
        );
        assert_eq!(query[1], ("page", "2".into()));
    }

    #[tokio::test]
    async fn all_pages_are_fetched_and_requested_status_is_preserved() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let route = warp::query::<HashMap<String, String>>()
            .and(warp::header::<String>("authorization"))
            .and(warp::header::<String>("xero-tenant-id"))
            .map(
                move |query: HashMap<String, String>, authorization: String, tenant: String| {
                    assert_eq!(authorization, "Bearer test-token");
                    assert_eq!(tenant, Uuid::nil().to_string());
                    assert_eq!(query["order"], "LeaveApplicationID ASC");
                    counted.fetch_add(1, Ordering::SeqCst);
                    let records: Vec<_> = match query["page"].as_str() {
                        "1" => (1..=100).map(application).collect(),
                        "2" => vec![application(101)],
                        "3" => vec![],
                        _ => panic!("unexpected page"),
                    };
                    warp::reply::json(&json!({"LeaveApplications": records}))
                },
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/leave", listener.local_addr().unwrap());
        let server = tokio::spawn(warp::serve(route).incoming(listener).run());
        let result = LeaveApplication::list_all_from(&crate::client::leave_test_client(), &url)
            .await
            .unwrap();
        server.abort();
        assert_eq!(result.len(), 101);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(
            result[0].leave_periods.as_ref().unwrap()[0].leave_period_status,
            Some(LeavePeriodStatus::Requested)
        );
    }

    #[tokio::test]
    async fn repeated_page_fails_instead_of_returning_partial_snapshot() {
        let route =
            warp::any().map(|| warp::reply::json(&json!({"LeaveApplications": [application(1)]})));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/leave", listener.local_addr().unwrap());
        let server = tokio::spawn(warp::serve(route).incoming(listener).run());
        let result =
            LeaveApplication::list_all_from(&crate::client::leave_test_client(), &url).await;
        server.abort();
        assert!(matches!(result, Err(crate::Error::InvalidParameter(_))));
    }

    #[tokio::test]
    async fn later_page_failure_does_not_return_first_page() {
        let route =
            warp::query::<HashMap<String, String>>().map(|query: HashMap<String, String>| {
                let (body, status) = if query["page"] == "1" {
                    (
                        json!({"LeaveApplications": [application(1)]}),
                        warp::http::StatusCode::OK,
                    )
                } else {
                    (
                        json!({"Type":"QueryParseException", "Message":"invalid query"}),
                        warp::http::StatusCode::BAD_REQUEST,
                    )
                };
                warp::reply::with_status(warp::reply::json(&body), status)
            });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/leave", listener.local_addr().unwrap());
        let server = tokio::spawn(warp::serve(route).incoming(listener).run());
        let result =
            LeaveApplication::list_all_from(&crate::client::leave_test_client(), &url).await;
        server.abort();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn modification_header_and_rate_limit_retry_use_shared_client() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let route = warp::header::<String>("if-modified-since").map(move |modified: String| {
            assert!(modified.starts_with("2026-09-10T00:00:00"));
            let status = if counted.fetch_add(1, Ordering::SeqCst) == 0 {
                warp::http::StatusCode::TOO_MANY_REQUESTS
            } else {
                warp::http::StatusCode::OK
            };
            warp::reply::with_header(
                warp::reply::with_status(
                    warp::reply::json(&json!({"LeaveApplications": []})),
                    status,
                ),
                "Retry-After",
                "0",
            )
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/leave", listener.local_addr().unwrap());
        let server = tokio::spawn(warp::serve(route).incoming(listener).run());
        let result = LeaveApplication::list_internal(
            &crate::client::leave_test_client(),
            &url,
            None,
            Some("2026-09-10T10:00:00+10:00".into()),
        )
        .await
        .unwrap();
        server.abort();
        assert!(result.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn invalid_parameters_fail_before_network_request() {
        let client = crate::client::leave_test_client();
        let parameters = ListParameters {
            page: Some(0),
            ..Default::default()
        };
        assert!(matches!(
            LeaveApplication::list_internal(&client, "invalid", Some(&parameters), None).await,
            Err(crate::Error::InvalidParameter(_))
        ));
        assert!(matches!(
            LeaveApplication::list_internal(
                &client,
                "invalid",
                None,
                Some("not a timestamp".into())
            )
            .await,
            Err(crate::Error::InvalidParameter(_))
        ));
    }
}
