//! Payslip entities for the Xero Payroll (AU) API.
//!
//! A payslip records the full earnings, deductions, tax, superannuation and
//! reimbursement detail for one employee within a pay run. See
//! <https://developer.xero.com/documentation/api/payrollau/payslip>.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Deserializers tolerant of Xero returning decimals as either JSON numbers or
/// quoted strings (the Payroll AU API is inconsistent about this).
pub(crate) mod flex {
    use serde::{Deserialize, Deserializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NumOrStr {
        Num(f64),
        Str(String),
    }

    /// Parse an optional `f64` from a number, a string, or null/absent.
    pub fn opt_f64<'de, D>(d: D) -> Result<Option<f64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<NumOrStr>::deserialize(d)? {
            Some(NumOrStr::Num(n)) => Some(n),
            Some(NumOrStr::Str(s)) => s.trim().parse::<f64>().ok(),
            None => None,
        })
    }

    /// Parse a required `f64` (number or string), defaulting to 0.0.
    pub fn f64_or_zero<'de, D>(d: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(opt_f64(d)?.unwrap_or(0.0))
    }
}

/// An earnings line on a payslip.
///
/// Used for `EarningsLines`, `TimesheetEarningsLines` and `LeaveEarningsLines`.
/// When updating a payslip, only `earnings_rate_id` and `number_of_units` are
/// sent — Xero applies the rate's configured (often fixed) rate per unit.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct EarningsLine {
    /// The earnings rate this line applies.
    #[serde(rename = "EarningsRateID")]
    pub earnings_rate_id: Uuid,
    /// Number of units (hours, km, days, …) for the line.
    #[serde(default, deserialize_with = "flex::f64_or_zero")]
    pub number_of_units: f64,
    /// Rate per unit as reported by Xero (read-only for fixed-rate items).
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub rate_per_unit: Option<f64>,
    /// Fixed amount for the line, when the earnings rate is a fixed amount.
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub fixed_amount: Option<f64>,
    /// Total amount for the line as calculated by Xero.
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub amount: Option<f64>,
    /// Whether the line is linked to a timesheet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_linked_to_timesheet: Option<bool>,
}

impl EarningsLine {
    /// Build a minimal update line: an earnings rate plus a unit count. Xero
    /// applies the rate's configured rate-per-unit (used for fixed-rate items
    /// such as the per-km motor vehicle allowance).
    #[must_use]
    pub fn units(earnings_rate_id: Uuid, number_of_units: f64) -> Self {
        Self {
            earnings_rate_id,
            number_of_units,
            rate_per_unit: None,
            fixed_amount: None,
            amount: None,
            is_linked_to_timesheet: None,
        }
    }
}

/// A deduction line on a payslip.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DeductionLine {
    #[serde(
        rename = "DeductionTypeID",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub deduction_type_id: Option<Uuid>,
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub amount: Option<f64>,
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub percentage: Option<f64>,
}

/// A reimbursement line on a payslip.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ReimbursementLine {
    #[serde(
        rename = "ReimbursementTypeID",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reimbursement_type_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub amount: Option<f64>,
}

/// A superannuation line on a payslip.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SuperannuationLine {
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub amount: Option<f64>,
    #[serde(
        rename = "SuperMembershipID",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub super_membership_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contribution_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_type: Option<String>,
}

/// A PAYG tax line on a payslip.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TaxLine {
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub amount: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_tax_type_name: Option<String>,
}

/// A leave accrual line on a payslip.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LeaveAccrualLine {
    #[serde(
        rename = "LeaveTypeID",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub leave_type_id: Option<Uuid>,
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub number_of_units: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_calculate: Option<bool>,
}

/// A full payslip, as returned by the Xero Payroll API.
///
/// The summary totals (`wages`, `tax`, `superannuation`, `net_pay`, …) and every
/// line array are captured exactly as Xero reports them.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Payslip {
    #[serde(rename = "PayslipID")]
    pub payslip_id: Uuid,
    #[serde(rename = "EmployeeID")]
    pub employee_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    // Summary totals, exactly as reported by Xero.
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub wages: Option<f64>,
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub deductions: Option<f64>,
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub reimbursements: Option<f64>,
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub tax: Option<f64>,
    #[serde(
        rename = "Super",
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub superannuation: Option<f64>,
    #[serde(
        default,
        deserialize_with = "flex::opt_f64",
        skip_serializing_if = "Option::is_none"
    )]
    pub net_pay: Option<f64>,

    // Line detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub earnings_lines: Option<Vec<EarningsLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timesheet_earnings_lines: Option<Vec<EarningsLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leave_earnings_lines: Option<Vec<EarningsLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deduction_lines: Option<Vec<DeductionLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reimbursement_lines: Option<Vec<ReimbursementLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superannuation_lines: Option<Vec<SuperannuationLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_lines: Option<Vec<TaxLine>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leave_accrual_lines: Option<Vec<LeaveAccrualLine>>,
}

/// Response wrapper for single-payslip API requests.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayslipResponse {
    pub payslip: Payslip,
}

/// Request body to update a payslip's earnings lines.
///
/// Earnings lines are merged by earnings rate — lines not supplied are left
/// untouched by Xero.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UpdatePayslip {
    #[serde(rename = "PayslipID")]
    pub payslip_id: Uuid,
    pub earnings_lines: Vec<EarningsLine>,
}
