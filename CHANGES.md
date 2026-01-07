# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0-alpha.13] - 2026-01-07

### Breaking Changes
- **DeserializationError variant**: Changed from tuple variant to struct variant
  - **Before**: `DeserializationError(serde_json::Error, Option<String>)`
  - **After**: `DeserializationError { source, response_body, error_span, context, entity_type, method, url, status_code }`
  - Pattern matching must now use struct syntax: `DeserializationError { source, context, .. }`

### Added
- **ResponseContext struct**: New struct capturing full HTTP response context for debugging
  - `url`: The URL that was called
  - `method`: HTTP method (GET, POST, PUT, DELETE)
  - `status_code`: HTTP status code returned
  - `response_body`: Raw response body (truncated to 2KB)
  - `entity_type`: The type being deserialized

- **Rich error diagnostics with miette**: DeserializationError now uses miette's `#[source_code]` and `#[label]` attributes
  - Response body displayed with error position highlighted
  - Beautiful terminal output for debugging parse failures
  - Includes help text and documentation URL

- **Error accessor methods**: New methods on `Error` enum
  - `response_context()`: Get full ResponseContext for DeserializationError
  - `response_body()`: Get response body (works for DeserializationError, NotFound, RateLimitExceeded)
  - `url()`: Get URL (works for DeserializationError, NotFound, RateLimitExceeded)
  - `status_code()`: Get HTTP status code

- **Constructor helper**: `Error::deserialization_error()` factory method
  - Creates DeserializationError with full HTTP context
  - Automatically calculates error span for miette highlighting

### Changed
- **handle_response() signature**: Now accepts `method: &str` parameter
  - Enables capturing HTTP method in error context
  - All execute_* methods updated to pass method string

- **Improved error messages**: DeserializationError now shows:
  - Entity type being deserialized
  - HTTP method and URL
  - HTTP status code
  - Parse error details with position

### Migration Guide

```rust
// BEFORE (v0.2.0-alpha.12 and earlier):
match error {
    xero_rs::Error::DeserializationError(serde_error, maybe_body) => {
        println!("Parse error: {}", serde_error);
        if let Some(body) = maybe_body {
            println!("Response: {}", body);
        }
    }
}

// AFTER (v0.2.0-alpha.13):
match error {
    xero_rs::Error::DeserializationError { source, context, .. } => {
        println!("Parse error: {}", source);
        println!("URL: {} {}", context.method, context.url);
        println!("Status: {}", context.status_code);
        println!("Response: {}", context.response_body);
    }
}

// Or use the new accessor methods:
if let Some(body) = error.response_body() {
    println!("Response: {}", body);
}
if let Some(url) = error.url() {
    println!("URL: {}", url);
}
```

## [0.2.0-alpha.6] - 2025-12-27

### Breaking Changes
- **Invoice.date field**: Changed from `Date` to `Option<Date>`
  - Some Xero invoices may not have a `DateString` field
  - Update code that accesses `invoice.date` to handle the Option

### Added
- **RateLimitType enum**: New enum to identify which rate limit was exceeded
  - `Minute`: Per-tenant minute limit (60 calls/minute)
  - `Daily`: Per-tenant daily limit (5000 calls/day)
  - `AppMinute`: App-wide minute limit (10,000 calls/minute)
  - Parsed from `X-Rate-Limit-Problem` header on 429 responses

- **Token expiry tracking**: Client now tracks token expiration time
  - `expires_at` field in internal token state
  - Enables proactive token refresh before requests fail

- **Proactive token refresh**: New `ensure_valid_token()` method
  - Refreshes token if expired or expiring within 60 seconds
  - Prevents failed requests due to token expiry
  - New `is_token_expiring()` helper method

- **Concurrency control**: Optional semaphore-based request limiting
  - `with_concurrency_limit(n)` builder method
  - Helps stay within Xero's 5 concurrent request limit
  - `without_concurrency_limit()` to disable

- **Validation error entity variants**: Added Invoice, Contact, and Item support
  - `ValidationExceptionElementObject::Invoice` with invoice_id, invoice_number
  - `ValidationExceptionElementObject::Contact` with contact_id, name
  - `ValidationExceptionElementObject::Item` with item_id, code
  - Previously these fell through to `Unknown` variant

### Changed
- **RateLimitExceeded error**: Now includes `limit_type` field
  - Identifies which specific rate limit was hit
  - Better error messages with limit type in display output

### Fixed
- **DELETE method rate limiting**: Fixed retry logic for DELETE requests on 429
- **Rate limit info persistence**: Rate limit headers now properly update client state

## [0.2.0-alpha.4] - 2025-11-20

### Breaking Changes
- **ValidationException error handling**: Removed `#[serde(default)]` from `elements` field in `ValidationException`
  - Previously, missing or invalid `Elements` arrays would silently default to empty vectors
  - Now, deserialization will fail if `Elements` field is missing or cannot be parsed
  - This improves error visibility and prevents silent failures when validation errors occur
  - **Migration**: Ensure your error handling accounts for potential deserialization errors from malformed API responses

### Added
- **Unknown entity type support**: Added `Unknown(serde_json::Value)` variant to `ValidationExceptionElementObject`
  - Provides forward compatibility for unsupported Xero entity types in validation errors
  - Preserves raw JSON for debugging and future implementation
- **Enhanced error documentation**: Added comprehensive doc comments to `ErrorType`, `ValidationException`, `ValidationExceptionElement`, and `ValidationExceptionElementObject`
  - Includes example JSON responses and usage notes
  - Documents the breaking change and its implications
- **Integration test infrastructure**: Created `tests/capture_validation_errors.rs`
  - Captures real Xero API validation error responses as JSON fixtures
  - Three test scenarios: missing contact, invalid contact, multiple validation errors
  - Run with: `cargo test --test capture_validation_errors -- --ignored --nocapture`
- **Decimal precision for unit prices**: Added `unitdp` parameter to `Invoice` and `Quote` `ListParameters`
  - Supports 4 decimal precision for unit prices (defaults to 2)
  - Use `.with_unitdp(4)` to request higher precision from Xero API

### Changed
- **ValidationExceptionElementObject matching**: Changed from tagged enum `#[serde(tag = "Type")]` to untagged `#[serde(untagged)]`
  - Matches based on field presence instead of discriminator
  - More flexible handling of Xero API responses (works with or without Type field)
- **Error type serialization**: Added `Serialize` derive to error types
  - Enables capturing and saving error responses as JSON fixtures for testing
  - Applies to: `ErrorType`, `ValidationExceptionElement`, `ValidationExceptionElementObject`, `Response`, `TimesheetValidationError`
- **Logging improvement**: Updated empty elements logging from ERROR to WARN level
  - Since deserialization now fails for missing Elements, reaching this code with empty array means Xero sent `Elements: []`
  - This is unusual but not necessarily a critical error

### Fixed
- **ValidationException deserialization bugs**:
  - Fixed field naming for `Elements` and `Timesheets` (now properly renamed from camelCase to PascalCase)
  - Fixed silent failure when Quote validation errors couldn't be deserialized
  - Improved error visibility for all validation exception scenarios

## [0.2.0-alpha.3] - 2024-09-20

### Added
- Purchase order update method and request struct
- Enhanced client method organization and refactoring

### Changed
- Bumped version to 0.2.0-alpha.3
- Code organization improvements via cargo clippy

## [0.2.0-alpha.2] - 2024-09-20

### Changed
- Bumped version to 0.2.0-alpha.2
- Client method refactoring and improvements

## [0.2.0-alpha.1] - 2024-09-20

### Changed
- Bumped version to 0.2.0-alpha.1
- Updated all dependencies to latest versions

---

## Release Notes

### For v0.2.0-alpha.4 Users

**Important**: This release includes a breaking change in error handling. If you're handling `ValidationException` errors:

1. **Before** (v0.2.0-alpha.3 and earlier):
```rust
if let Err(Error::API(api_error)) = result {
    if let ErrorType::ValidationException { elements, .. } = api_error.error {
        // elements was always a Vec, even if deserialization failed
        // Could be empty even when Xero returned validation errors
        for element in elements {
            // Handle validation errors
        }
    }
}
```

2. **After** (v0.2.0-alpha.4):
```rust
if let Err(error) = result {
    match error {
        Error::API(api_error) => {
            if let ErrorType::ValidationException { elements, .. } = api_error.error {
                // elements is guaranteed to contain actual validation data
                // If Elements field was missing/invalid, you'll get DeserializationError instead
                for element in elements {
                    // Handle validation errors
                }
            }
        }
        Error::DeserializationError(err, raw_response) => {
            // New case: handle malformed validation responses
            // Check raw_response for debugging
        }
        _ => { /* other errors */ }
    }
}
```

**Benefits**:
- More reliable error handling
- No silent failures
- Unknown entity types captured for debugging
- Better error visibility

[0.2.0-alpha.4]: https://github.com/slickbench/xero-rs/compare/v0.2.0-alpha.3...v0.2.0-alpha.4
[0.2.0-alpha.3]: https://github.com/slickbench/xero-rs/compare/v0.2.0-alpha.2...v0.2.0-alpha.3
[0.2.0-alpha.2]: https://github.com/slickbench/xero-rs/compare/v0.2.0-alpha.1...v0.2.0-alpha.2
[0.2.0-alpha.1]: https://github.com/slickbench/xero-rs/releases/tag/v0.2.0-alpha.1
