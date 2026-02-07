# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0-alpha.23] - 2026-02-07

### Added
- `UnitDp` enum for type-safe unit decimal places configuration (`UnitDp::Two`, `UnitDp::Four`)
- `Client::with_unitdp()` builder method to set a client-wide default `unitdp` that is automatically applied to all applicable endpoints (invoices, items, quotes)
- `unitdp` support for Item mutations (create, update, update_or_create) - previously only supported on GET
- `unitdp` automatically applied to single-entity GET requests (`get`, `get_by_code`) for invoices, items, and quotes

### Changed
- `ListParameters.unitdp` fields on invoices, items, and quotes changed from `Option<u8>` to `Option<UnitDp>` (**breaking**)
- `with_unitdp()` builder methods on `ListParameters` now take `UnitDp` instead of `u8` (**breaking**)
- `MutationOptions` removed from the public API (**breaking**) - callers no longer pass it manually; `unitdp` is configured once on the `Client` and applied automatically

### Removed
- `MutationOptions` is no longer publicly exported - it is now `pub(crate)`
- `options` parameter removed from `InvoicesApi::create()`, `InvoicesApi::update()`, `InvoicesApi::update_or_create()` (**breaking**)
- `options` parameter removed from `QuotesApi::create()`, `QuotesApi::update()`, `QuotesApi::update_or_create()` (**breaking**)

### Migration Guide

Before:
```rust
let client = Client::from_client_credentials(key_pair, None).await?;
let options = MutationOptions { unitdp: Some(4) };
client.invoices().create(&builder, &options).await?;
client.invoices().list(ListParameters::default().with_unitdp(4)).await?;
```

After:
```rust
use xero_rs::UnitDp;

let client = Client::from_client_credentials(key_pair, None)
    .await?
    .with_unitdp(UnitDp::Four);

// unitdp=4 applied automatically to all applicable requests:
client.invoices().create(&builder).await?;
client.invoices().list(ListParameters::default()).await?;
// Per-request override still works:
client.invoices().list(ListParameters::default().with_unitdp(UnitDp::Two)).await?;
```

## [0.2.0-alpha.22] - 2026-02-06

### Added
- `MutationOptions` for `unitdp` query param on PUT/POST requests

## [0.2.0-alpha.21] - 2026-02-06

### Fixed
- Log full `ValidationException` details for debugging
- Make `ValidationException` Elements field optional for payroll API
- Implement concurrent rate limit handling
- Make contact field optional in entity builders
