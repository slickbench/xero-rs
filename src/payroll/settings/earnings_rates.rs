use serde::Deserialize;
use uuid::Uuid;

use super::leave_types::LeaveType;

pub const ENDPOINT: &str = "https://api.xero.com/payroll.xro/1.0/PayItems";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EarningsRate {
    #[serde(rename = "EarningsRateID")]
    pub earnings_rate_id: Uuid,
    pub name: String,
    pub earnings_type: String,
    pub rate_type: String,
    #[serde(default)]
    pub type_of_units: Option<String>,
    pub account_code: Option<String>,
    pub multiplier: Option<f64>,
    #[serde(default)]
    pub is_exempt_from_tax: Option<bool>,
    #[serde(default)]
    pub is_exempt_from_super: Option<bool>,
    #[serde(default)]
    pub is_reportable_as_w1: Option<bool>,
    #[serde(default)]
    pub accrue_leave: Option<bool>,
    pub updated_date_utc: Option<String>,
    pub current_record: Option<bool>,
    pub employment_termination_payment_type: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayItems {
    pub earnings_rates: Vec<EarningsRate>,
    #[serde(default)]
    pub leave_types: Vec<LeaveType>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ListResponse {
    #[allow(dead_code)]
    pub pay_items: PayItems,
}
