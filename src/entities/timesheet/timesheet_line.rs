use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct TimesheetLine {
    pub timesheet_line_id: Uuid,
    pub date: String,
    pub earnings_rate_id: Uuid,
    pub number_of_units: Vec<f64>,
}
