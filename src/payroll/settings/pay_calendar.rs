use crate::utils::date_format::{xero_date_format, xero_date_format_option};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use time::Date;
use uuid::Uuid;

/// Calendar types supported by the Xero Payroll API
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CalendarType {
    /// Weekly payment schedule
    Weekly,
    /// Fortnightly payment schedule (every two weeks)
    Fortnightly,
    /// Monthly payment schedule
    Monthly,
    /// Four-weekly payment schedule
    FourWeekly,
    /// Twice-monthly payment schedule
    TwiceMonthly,
    /// Quarterly payment schedule
    Quarterly,
}

impl FromStr for CalendarType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "WEEKLY" => Ok(CalendarType::Weekly),
            "FORTNIGHTLY" => Ok(CalendarType::Fortnightly),
            "MONTHLY" => Ok(CalendarType::Monthly),
            "FOURWEEKLY" => Ok(CalendarType::FourWeekly),
            "TWICEMONTHLY" => Ok(CalendarType::TwiceMonthly),
            "QUARTERLY" => Ok(CalendarType::Quarterly),
            _ => Err(format!("Unknown calendar type: {s}")),
        }
    }
}

/// Represents a pay calendar in the Xero Payroll API
///
/// Pay calendars define the pay periods for employees.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayCalendar {
    /// The unique identifier for the pay calendar
    #[serde(rename = "PayrollCalendarID")]
    pub pay_calendar_id: Uuid,
    /// The name of the pay calendar
    pub name: String,
    /// The type of calendar (e.g., "WEEKLY", "FORTNIGHTLY", "MONTHLY")
    #[serde(with = "calendar_type_string")]
    pub calendar_type: CalendarType,
    /// The start date for the pay period
    #[serde(with = "xero_date_format")]
    pub start_date: Date,
    /// The payment date for the pay period
    #[serde(with = "xero_date_format")]
    pub payment_date: Date,
    /// The date and time when the pay calendar was last updated
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: Option<String>,
    /// The reference date for the pay calendar
    #[serde(default, deserialize_with = "xero_date_format_option::deserialize")]
    pub reference_date: Option<Date>,
}

impl PayCalendar {
    /// Returns the end date of the pay period, which is the day before the payment date
    #[must_use]
    pub fn end_date(&self) -> Date {
        self.payment_date.saturating_sub(time::Duration::days(1))
    }
}

/// Response wrapper for pay calendar API requests
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct PayCalendarResponse {
    /// List of pay calendars returned by the API
    pub payroll_calendars: Vec<PayCalendar>,
}

/// Represents the data needed to create a new pay calendar
///
/// This struct is used when creating a new pay calendar via the Xero Payroll API.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreatePayCalendar {
    /// The name of the pay calendar
    pub name: String,
    /// The type of calendar (e.g., "WEEKLY", "FORTNIGHTLY", "MONTHLY")
    #[serde(with = "calendar_type_string")]
    pub calendar_type: CalendarType,
    /// The start date for the pay period
    #[serde(with = "xero_date_format")]
    pub start_date: Date,
    /// The payment date for the pay period
    #[serde(with = "xero_date_format")]
    pub payment_date: Date,
}

/// Request wrapper for creating a pay calendar
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreatePayCalendarRequest {
    pub payroll_calendars: Vec<CreatePayCalendar>,
}

// Serialization of calendar type to/from string
mod calendar_type_string {
    use super::CalendarType;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(calendar_type: &CalendarType, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{calendar_type:?}").to_uppercase())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<CalendarType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<CalendarType>().map_err(serde::de::Error::custom)
    }
}
