use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayCalendar {
    #[serde(rename = "PayrollCalendarID")]
    pub pay_calendar_id: Uuid,
    pub name: String,
    pub calendar_type: String,
    pub start_date: String,
    pub payment_date: String,
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: Option<String>,
    pub reference_date: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayCalendarResponse {
    pub payroll_calendars: Vec<PayCalendar>,
}
