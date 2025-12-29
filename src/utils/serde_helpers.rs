use serde::{Deserialize, Deserializer};

/// Deserializes a value, treating empty strings as None.
/// Useful for Xero API fields that return "" instead of null.
pub fn empty_string_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrValue<T> {
        String(String),
        Value(T),
    }

    match Option::<StringOrValue<T>>::deserialize(deserializer)? {
        None => Ok(None),
        Some(StringOrValue::String(s)) if s.is_empty() => Ok(None),
        Some(StringOrValue::String(s)) => Err(serde::de::Error::custom(format!(
            "unexpected string value: {s}"
        ))),
        Some(StringOrValue::Value(v)) => Ok(Some(v)),
    }
}
