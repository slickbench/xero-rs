# xero-rs

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![Coverage][codecov-badge]][codecov-url]

[crates-badge]: https://img.shields.io/crates/v/xero-rs.svg
[crates-url]: https://crates.io/crates/xero-rs
[docs-badge]: https://docs.rs/xero-rs/badge.svg
[docs-url]: https://docs.rs/xero-rs
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/slickbench/xero-rs/actions/workflows/tests.yml/badge.svg
[actions-url]: https://github.com/slickbench/xero-rs/actions/workflows/tests.yml
[codecov-badge]: https://codecov.io/gh/slickbench/xero-rs/branch/main/graph/badge.svg?token=INFT14K6KW
[codecov-url]: https://codecov.io/gh/slickbench/xero-rs

## Description

A Xero API client library for Rust. This library is in very early days and the API is **not** stable, it may change without notice.

This was put together as part of the requirements for a private project so I will be implementing features as-needed, but all contributions are welcome.

## Features

- Client credential & code flow authorization support
- Generic GET, PUT, and POST methods for custom requests
- Type-safe API endpoint construction via `XeroEndpoint` enum
- Uses [rust_decimal](https://github.com/paupino/rust-decimal) for storing prices/decimal values
- Rich error diagnostics via [miette](https://github.com/zkat/miette) integration
- Well tested (that's the goal, at least)

## Error Handling

`xero-rs` uses [miette](https://github.com/zkat/miette) for rich error diagnostics. The `Error` type implements the `Diagnostic` trait, which means you get detailed error reports with context and help text.

```rust
// Example of handling errors with miette
use xero_rs::error::Result;

fn do_something() -> Result<()> {
    // If this fails, you'll get a rich diagnostic error
    let client = xero_rs::Client::from_client_credentials(...)?;
    
    // No need to call .into_diagnostic() when using xero_rs::error::Result
    Ok(())
}
```

When using with miette's own `Result` type, you'll need to use `.into_diagnostic()`:

```rust
use miette::{Result, IntoDiagnostic};

fn do_something() -> Result<()> {
    // Converting to miette's Result requires .into_diagnostic()
    let client = xero_rs::Client::from_client_credentials(...).into_diagnostic()?;
    
    Ok(())
}
```

## Currently Implemented

This has been implemented so far:

- OAuth2 Authentication (Client Credentials, Authorization Code Flow)
- Type-safe API URL construction with `XeroEndpoint`
- List authorized connections (tennants)
- Quotes
  - List
  - Get by ID
- Invoices
  - List
  - Get by ID
- Purchase Orders
  - List
  - Get by ID
  - Create
- Contacts
  - List
