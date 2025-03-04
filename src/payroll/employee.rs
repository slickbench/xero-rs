use serde::Deserialize;
use uuid::Uuid;

use crate::{error::Result, Client};

pub const ENDPOINT: &str = "https://api.xero.com/payroll.xro/1.0/Employees";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Employee {
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub email: Option<String>,
    pub status: String,
    #[serde(rename = "PayrollCalendarID")]
    pub payroll_calendar_id: Option<Uuid>,
    pub date_of_birth: Option<String>,
    pub gender: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub start_date: Option<String>,
    #[serde(rename = "OrdinaryEarningsRateID")]
    pub ordinary_earnings_rate_id: Option<Uuid>,
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: Option<String>,
    #[serde(rename = "IsSTP2Qualified")]
    pub is_stp2_qualified: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListResponse {
    employees: Vec<Employee>,
}

/// Retrieve a list of employees.
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<Employee>> {
    let response: ListResponse = client.get(ENDPOINT, Vec::<String>::default()).await?;
    Ok(response.employees)
}
