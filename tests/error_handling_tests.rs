use serde_json::json;
use xero_rs::error::{ErrorType, Response as ErrorResponse};

#[test]
fn test_query_parse_exception_handling() {
    // Test that QueryParseException can be deserialized
    let error_json = json!({
        "ErrorNumber": 16,
        "Type": "QueryParseException",
        "Message": "Unterminated string literal"
    });

    let result: Result<ErrorResponse, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize QueryParseException: {:?}",
        result.err()
    );

    let error_response = result.unwrap();
    match error_response.error {
        ErrorType::QueryParseException => {
            // Success - error type was recognized
        }
        _ => panic!(
            "Expected QueryParseException, got {:?}",
            error_response.error
        ),
    }
}

#[test]
fn test_validation_exception_handling() {
    // Test that ValidationException can be deserialized
    let error_json = json!({
        "ErrorNumber": 10,
        "Type": "ValidationException",
        "Message": "A validation error occurred"
    });

    let result: Result<ErrorResponse, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize ValidationException: {:?}",
        result.err()
    );

    let error_response = result.unwrap();
    match &error_response.error {
        ErrorType::ValidationException { .. } => {
            // Success - error type was recognized
        }
        _ => panic!(
            "Expected ValidationException, got {:?}",
            error_response.error
        ),
    }
}

#[test]
fn test_error_display_formatting() {
    // Test the Display implementation for better error messages
    let error_response = ErrorResponse {
        error_number: 16,
        message: "Unterminated string literal".to_string(),
        error: ErrorType::QueryParseException,
    };

    let display_text = format!("{}", error_response);
    assert!(display_text.contains("Xero API Error (16): Unterminated string literal"));
    assert!(display_text.contains("The query string could not be parsed"));
}

#[test]
fn test_all_error_types_deserialize() {
    // Test that all error types can be deserialized
    let error_types = vec![
        (
            "ValidationException",
            json!({"Type": "ValidationException", "ErrorNumber": 10, "Message": "Test"}),
        ),
        (
            "PostDataInvalidException",
            json!({"Type": "PostDataInvalidException", "ErrorNumber": 11, "Message": "Test"}),
        ),
        (
            "QueryParseException",
            json!({"Type": "QueryParseException", "ErrorNumber": 16, "Message": "Test"}),
        ),
        (
            "ObjectNotFoundException",
            json!({"Type": "ObjectNotFoundException", "ErrorNumber": 17, "Message": "Test"}),
        ),
        (
            "OrganisationOfflineException",
            json!({"Type": "OrganisationOfflineException", "ErrorNumber": 18, "Message": "Test"}),
        ),
        (
            "UnauthorisedException",
            json!({"Type": "UnauthorisedException", "ErrorNumber": 19, "Message": "Test"}),
        ),
        (
            "NoDataProcessedException",
            json!({"Type": "NoDataProcessedException", "ErrorNumber": 20, "Message": "Test"}),
        ),
        (
            "UnsupportedMediaTypeException",
            json!({"Type": "UnsupportedMediaTypeException", "ErrorNumber": 21, "Message": "Test"}),
        ),
        (
            "MethodNotAllowedException",
            json!({"Type": "MethodNotAllowedException", "ErrorNumber": 22, "Message": "Test"}),
        ),
        (
            "InternalServerException",
            json!({"Type": "InternalServerException", "ErrorNumber": 23, "Message": "Test"}),
        ),
        (
            "NotImplementedException",
            json!({"Type": "NotImplementedException", "ErrorNumber": 24, "Message": "Test"}),
        ),
        (
            "NotAvailableException",
            json!({"Type": "NotAvailableException", "ErrorNumber": 25, "Message": "Test"}),
        ),
        (
            "RateLimitExceededException",
            json!({"Type": "RateLimitExceededException", "ErrorNumber": 26, "Message": "Test"}),
        ),
        (
            "SystemUnavailableException",
            json!({"Type": "SystemUnavailableException", "ErrorNumber": 27, "Message": "Test"}),
        ),
    ];

    for (error_type, json_value) in error_types {
        let result: Result<ErrorResponse, _> = serde_json::from_value(json_value);
        assert!(
            result.is_ok(),
            "Failed to deserialize {}: {:?}",
            error_type,
            result.err()
        );
    }
}
