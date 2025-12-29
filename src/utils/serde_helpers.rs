use serde::{Deserialize, Deserializer, de::IntoDeserializer};

/// Deserializes a value, treating empty strings as None.
/// Useful for Xero API fields that return "" instead of null.
pub fn empty_string_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    // First deserialize as Option<String> to check for empty string
    let opt: Option<String> = Option::deserialize(deserializer)?;

    match opt {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => {
            // Try to deserialize the string as T
            T::deserialize(s.into_deserializer()).map(Some)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::account::BankAccountType;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestAccount {
        name: String,
        #[serde(default, deserialize_with = "empty_string_as_none")]
        bank_account_type: Option<BankAccountType>,
    }

    #[test]
    fn test_empty_string_becomes_none() {
        let json = r#"{"name": "Test Account", "bank_account_type": ""}"#;
        let account: TestAccount = serde_json::from_str(json).unwrap();
        assert_eq!(account.name, "Test Account");
        assert_eq!(account.bank_account_type, None);
    }

    #[test]
    fn test_null_becomes_none() {
        let json = r#"{"name": "Test Account", "bank_account_type": null}"#;
        let account: TestAccount = serde_json::from_str(json).unwrap();
        assert_eq!(account.bank_account_type, None);
    }

    #[test]
    fn test_missing_field_becomes_none() {
        let json = r#"{"name": "Test Account"}"#;
        let account: TestAccount = serde_json::from_str(json).unwrap();
        assert_eq!(account.bank_account_type, None);
    }

    #[test]
    fn test_valid_enum_value_bank() {
        let json = r#"{"name": "Test Account", "bank_account_type": "BANK"}"#;
        let account: TestAccount = serde_json::from_str(json).unwrap();
        assert_eq!(account.bank_account_type, Some(BankAccountType::Bank));
    }

    #[test]
    fn test_valid_enum_value_creditcard() {
        let json = r#"{"name": "Test Account", "bank_account_type": "CREDITCARD"}"#;
        let account: TestAccount = serde_json::from_str(json).unwrap();
        assert_eq!(account.bank_account_type, Some(BankAccountType::CreditCard));
    }

    #[test]
    fn test_valid_enum_value_paypal() {
        let json = r#"{"name": "Test Account", "bank_account_type": "PAYPAL"}"#;
        let account: TestAccount = serde_json::from_str(json).unwrap();
        assert_eq!(account.bank_account_type, Some(BankAccountType::PayPal));
    }

    #[test]
    fn test_invalid_enum_value_fails() {
        let json = r#"{"name": "Test Account", "bank_account_type": "INVALID"}"#;
        let result: Result<TestAccount, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
