use serde::{self, Deserialize, Deserializer, Serializer};
use time::{Date, OffsetDateTime, macros::format_description};

// Function to handle Xero's .NET JSON date format (/Date(timestamp)/)
// Also handles date strings that may include time components
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
            let date = OffsetDateTime::from_unix_timestamp(seconds)
                .map_err(|e| format!("Invalid timestamp: {e}"))?
                .date();
            return Ok(date);
        }
    }
    
    // If the string contains a 'T', it might be a datetime string - extract just the date part
    if date_str.contains('T')
        && let Some(date_part) = date_str.split('T').next() {
            // Try to parse just the date part
            let format = format_description!("[year]-[month]-[day]");
            if let Ok(date) = Date::parse(date_part, &format) {
                return Ok(date);
            }
        }
    
    // Try as plain ISO format
    let format = format_description!("[year]-[month]-[day]");
    Date::parse(date_str, &format)
        .map_err(|e| format!("Failed to parse date '{date_str}': {e}"))
}

// Function to handle Xero's .NET JSON datetime format (/Date(timestamp)/)
// Also tries to handle various other formats Xero might return
pub fn parse_dotnet_datetime(datetime_str: &str) -> Result<OffsetDateTime, String> {
    // Extract the timestamp from the .NET date format
    if datetime_str.starts_with("/Date(") && datetime_str.ends_with(")/") {
        let timestamp_str = datetime_str
            .trim_start_matches("/Date(")
            .trim_end_matches(")/")
            .split('+')
            .next()
            .unwrap_or(datetime_str);
        
        // Try to parse as a timestamp (milliseconds since epoch)
        if let Ok(timestamp) = timestamp_str.parse::<i64>() {
            // Convert to seconds and create an OffsetDateTime
            let seconds = timestamp / 1000;
            let datetime = OffsetDateTime::from_unix_timestamp(seconds)
                .map_err(|e| format!("Invalid timestamp: {e}"))?;
            return Ok(datetime);
        }
    }
    
    // Try various datetime formats that Xero might return
    
    // Standard RFC3339
    let rfc3339 = time::format_description::well_known::Rfc3339;
    if let Ok(dt) = OffsetDateTime::parse(datetime_str, &rfc3339) {
        return Ok(dt);
    }
    
    // Format with fractional seconds but no timezone (assume UTC)
    // e.g. "2025-03-03T06:17:25.8448470"
    if datetime_str.contains('T') && datetime_str.contains('.') && !datetime_str.contains('+') && !datetime_str.contains('Z') {
        // Try to parse with a custom format
        let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]");
        if let Ok(dt) = time::PrimitiveDateTime::parse(datetime_str, &format) {
            // Convert to OffsetDateTime at UTC
            return Ok(dt.assume_utc());
        }
    }
    
    // ISO format with T but no fractional seconds and no timezone
    if datetime_str.contains('T') && !datetime_str.contains('.') && !datetime_str.contains('+') && !datetime_str.contains('Z') {
        let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]");
        if let Ok(dt) = time::PrimitiveDateTime::parse(datetime_str, &format) {
            // Convert to OffsetDateTime at UTC
            return Ok(dt.assume_utc());
        }
    }
    
    Err(format!("Failed to parse datetime '{datetime_str}': no matching format"))
}

// Serialization module for time::Date
pub mod xero_date_format {
    use super::{Date, Deserialize, Deserializer, Serializer, format_description, parse_dotnet_date, serde};

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

// Optional date serialization module
pub mod xero_date_format_option {
    use super::{Date, Deserialize, Deserializer, Serializer, format_description, serde};
    
    pub fn serialize<S>(date: &Option<Date>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(date) => {
                let formatted = date
                    .format(&format_description!("[year]-[month]-[day]"))
                    .map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&formatted)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        
        match opt {
            Some(s) if !s.is_empty() => {
                // Try to parse the date string
                let date_result = super::parse_dotnet_date(&s);
                match date_result {
                    Ok(date) => Ok(Some(date)),
                    Err(_) => Ok(None), // Return None if parsing fails
                }
            }
            _ => Ok(None),
        }
    }
}

// Date-time serialization modules for time::OffsetDateTime
pub mod xero_datetime_format {
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(datetime: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Format as RFC3339 (ISO8601 with timezone)
        let formatted = datetime
            .format(&Rfc3339)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let datetime_str = String::deserialize(deserializer)?;
        
        // Try to parse using our flexible parser
        super::parse_dotnet_datetime(&datetime_str)
            .map_err(serde::de::Error::custom)
    }
}

// Optional OffsetDateTime serialization
pub mod xero_datetime_format_option {
    use time::{OffsetDateTime, format_description::well_known::Rfc3339};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(datetime: &Option<OffsetDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match datetime {
            Some(dt) => {
                let formatted = dt
                    .format(&Rfc3339)
                    .map_err(serde::ser::Error::custom)?;
                serializer.serialize_str(&formatted)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        
        match opt {
            Some(s) if !s.is_empty() => {
                // Try to parse using our flexible parser
                match super::parse_dotnet_datetime(&s) {
                    Ok(dt) => Ok(Some(dt)),
                    Err(_) => Ok(None), // Return None if parsing fails
                }
            }
            _ => Ok(None),
        }
    }
} 