# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
