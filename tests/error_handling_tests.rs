use serde_json::json;
use xero_rs::error::{self, ErrorType, Response as ErrorResponse};

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
    // Test that ValidationException can be deserialized with Elements array
    let error_json = json!({
        "ErrorNumber": 10,
        "Type": "ValidationException",
        "Message": "A validation error occurred",
        "Elements": [{
            "QuoteID": "efcef70f-f4f9-4baf-83b6-b5eac086c91b",
            "Status": "ACCEPTED",
            "ValidationErrors": [{
                "Message": "Contact requires a valid ContactId or ContactName"
            }]
        }]
    });

    let result: Result<ErrorResponse, _> = serde_json::from_value(error_json);
    assert!(
        result.is_ok(),
        "Failed to deserialize ValidationException: {:?}",
        result.err()
    );

    let error_response = result.unwrap();
    match &error_response.error {
        ErrorType::ValidationException { elements, .. } => {
            // Verify elements were parsed correctly
            assert_eq!(elements.len(), 1, "Expected 1 validation element");
            assert_eq!(
                elements[0].validation_errors.len(),
                1,
                "Expected 1 validation error"
            );
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
        error_number: Some(16),
        status: Some(400),
        title: Some("Unterminated string literal".to_string()),
        detail: Some("Unterminated string literal".to_string()),
        instance: None,
        message: Some("Unterminated string literal".to_string()),
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
            json!({
                "Type": "ValidationException",
                "ErrorNumber": 10,
                "Message": "Test",
                "Elements": []
            }),
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

/// Xero 500 responses often lack the "Type" field, so error::Response
/// deserialization fails. The client catch-all for 5xx should detect this
/// and produce Error::ServerError instead of Error::DeserializationError.
#[test]
fn test_500_without_type_field_fails_to_parse_as_error_response() {
    // Realistic Xero 500 body — no Type field
    let body = json!({
        "Message": "An error occurred",
        "ErrorNumber": 0
    });

    let result: Result<ErrorResponse, _> = serde_json::from_value(body);
    assert!(
        result.is_err(),
        "Expected error::Response to fail without Type field, but it parsed: {:?}",
        result.unwrap()
    );
}

/// When a 5xx response DOES include a valid Type field, the client should
/// still parse it as Error::API (existing behaviour).
#[test]
fn test_500_with_type_field_parses_as_error_response() {
    let body = json!({
        "Type": "InternalServerException",
        "ErrorNumber": 23,
        "Message": "An internal error occurred"
    });

    let result: Result<ErrorResponse, _> = serde_json::from_value(body);
    assert!(
        result.is_ok(),
        "error::Response should parse 500 with valid Type: {:?}",
        result.err()
    );

    let response = result.unwrap();
    assert!(
        matches!(response.error, ErrorType::InternalServerException),
        "Expected InternalServerException, got {:?}",
        response.error
    );
}

/// Verify ServerError variant fields are accessible via accessor methods.
#[test]
fn test_server_error_variant_accessors() {
    use tracing_error::SpanTrace;

    let error = error::Error::ServerError {
        status_code: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        message: "Something went wrong".to_string(),
        response_body: Some(r#"{"Message":"Something went wrong"}"#.to_string()),
        url: "https://api.xero.com/api.xro/2.0/Quotes".to_string(),
        span_trace: SpanTrace::capture(),
    };

    assert_eq!(
        error.status_code(),
        Some(reqwest::StatusCode::INTERNAL_SERVER_ERROR)
    );
    assert_eq!(error.url(), Some("https://api.xero.com/api.xro/2.0/Quotes"));
    assert!(error.response_body().is_some());
    assert!(error.span_trace().is_some());

    let display = format!("{error}");
    assert!(
        display.contains("500"),
        "Display should include status code: {display}"
    );
    assert!(
        display.contains("Something went wrong"),
        "Display should include message: {display}"
    );
}
