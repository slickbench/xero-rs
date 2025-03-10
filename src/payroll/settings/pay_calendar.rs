use serde::{Deserialize, Serialize};
use uuid::Uuid;
use time::Date;
use std::str::FromStr;

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
            _ => Err(format!("Unknown calendar type: {}", s)),
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
        serializer.serialize_str(&format!("{:?}", calendar_type).to_uppercase())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<CalendarType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<CalendarType>().map_err(serde::de::Error::custom)
    }
}

// Serialization helpers for Xero date formats
mod xero_date_format {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use time::{Date, macros::format_description};

    // Function to handle Xero's .NET JSON date format (/Date(timestamp)/)
    pub fn parse_dotnet_date(date_str: &str) -> Result<Date, String> {
        // Extract the timestamp from the .NET date format
        if date_str.starts_with("/Date(") && date_str.ends_with(")/") {
            let timestamp_str = date_str
                .trim_start_matches("/Date(")
                .trim_end_matches(")/")
                .split('+')
                .next()
                .unwrap_or(date_str);
            
            // Try to parse as a timestamp (milliseconds since epoch)
            if let Ok(timestamp) = timestamp_str.parse::<i64>() {
                // Convert to seconds and create a Date
                let seconds = timestamp / 1000;
                let date = time::OffsetDateTime::from_unix_timestamp(seconds)
                    .map_err(|e| format!("Invalid timestamp: {}", e))?
                    .date();
                return Ok(date);
            }
        }
        
        // If not a .NET date format, try as ISO format
        let format = format_description!("[year]-[month]-[day]");
        Date::parse(date_str, &format)
            .map_err(|e| format!("Failed to parse date '{}': {}", date_str, e))
    }

    pub fn serialize<S>(date: &Date, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Format as ISO 8601 date (YYYY-MM-DD)
        let formatted = date
            .format(&format_description!("[year]-[month]-[day]"))
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
    where
        D: Deserializer<'de>,
    {
        let date_str = String::deserialize(deserializer)?;
        
        // Try to parse the date string
        parse_dotnet_date(&date_str).map_err(serde::de::Error::custom)
    }
}

// Optional date serialization
mod xero_date_format_option {
    use serde::{self, Deserialize, Deserializer};
    use time::Date;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        
        match opt {
            Some(s) if !s.is_empty() => {
                // Try to parse the date string
                let date_result = super::xero_date_format::parse_dotnet_date(&s);
                match date_result {
                    Ok(date) => Ok(Some(date)),
                    Err(_) => Ok(None), // Return None if parsing fails
                }
            }
            _ => Ok(None),
        }
    }
}
