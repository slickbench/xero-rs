//! Pay run entities for the Xero Payroll (AU) API.
//!
//! Pay runs group the payslips paid to employees for a single pay period on a
//! payroll calendar. See <https://developer.xero.com/documentation/api/payrollau/payruns>.

use crate::utils::date_format::xero_date_format_option;
use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

/// A pay run in the Xero Payroll API.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayRun {
    /// Unique identifier for the pay run.
    #[serde(rename = "PayRunID")]
    pub pay_run_id: Uuid,
    /// The payroll calendar the pay run belongs to.
    #[serde(
        rename = "PayrollCalendarID",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub payroll_calendar_id: Option<Uuid>,
    /// Start date of the pay period (parsed from Xero's `/Date()/` format).
    #[serde(default, with = "xero_date_format_option")]
    pub pay_run_period_start_date: Option<Date>,
    /// End date of the pay period (parsed from Xero's `/Date()/` format).
    #[serde(default, with = "xero_date_format_option")]
    pub pay_run_period_end_date: Option<Date>,
    /// Status of the pay run, e.g. `DRAFT` or `POSTED`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pay_run_status: Option<String>,
    /// Payslip summaries. Populated when a single pay run is fetched by ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payslips: Option<Vec<PayslipSummary>>,
}

/// A summary of a payslip as returned in a pay run's `Payslips` array.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayslipSummary {
    /// Unique identifier for the payslip.
    #[serde(rename = "PayslipID")]
    pub payslip_id: Uuid,
    /// The employee the payslip belongs to.
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,
    /// Employee first name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    /// Employee last name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    /// Total wages/earnings for the payslip.
    #[serde(
        default,
        deserialize_with = "super::payslip::flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub wages: Option<f64>,
    /// Total deductions for the payslip.
    #[serde(
        default,
        deserialize_with = "super::payslip::flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub deductions: Option<f64>,
    /// Total reimbursements for the payslip.
    #[serde(
        default,
        deserialize_with = "super::payslip::flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub reimbursements: Option<f64>,
    /// Total PAYG tax withheld.
    #[serde(
        default,
        deserialize_with = "super::payslip::flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub tax: Option<f64>,
    /// Total superannuation.
    #[serde(
        rename = "Super",
        default,
        deserialize_with = "super::payslip::flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub superannuation: Option<f64>,
    /// Net pay.
    #[serde(
        default,
        deserialize_with = "super::payslip::flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub net_pay: Option<f64>,
}

/// Response wrapper for pay run API requests.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayRunResponse {
    /// The pay runs returned by the API.
    pub pay_runs: Vec<PayRun>,
}

/// Request body used to create a new pay run for a payroll calendar.
///
/// Xero automatically derives the pay period dates from the calendar.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreatePayRun {
    /// The payroll calendar to create the pay run against.
    #[serde(rename = "PayrollCalendarID")]
    pub payroll_calendar_id: Uuid,
}
