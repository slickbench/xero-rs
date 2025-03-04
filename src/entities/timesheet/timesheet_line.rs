use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TimesheetLine {
    #[serde(rename = "TimesheetLineID", skip_serializing_if = "Option::is_none")]
    pub timesheet_line_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(rename = "EarningsRateID")]
    pub earnings_rate_id: Uuid,
    pub number_of_units: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_date_utc: Option<String>,
}
