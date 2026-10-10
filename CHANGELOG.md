# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0-alpha.28]

### Added
- `purchase_orders().get_pdf(id)`, `get_history(id)` and `create_history(id, details)`.
- `Client::get_bytes(endpoint, accept)`: binary downloads (PDFs, attachments) with the same pacing, token refresh, rate-limit retry and error mapping as JSON requests.
- `entities::document`: the one `HistoryRecord` type and the history, PDF and attachment calls every document shares. `invoice::`, `quote::`, `item::` and `purchase_order::HistoryRecord` re-export it.
- `examples/document_pdf.rs`: download a purchase order, invoice or quote PDF.

### Fixed
- `get_pdf` for invoices and quotes never returned a PDF: it asked for `{Resource}/{id}/pdf`, which the live API answers with a 404 (the OpenAPI spec and SDKs name that path), and sent `Accept: application/json`. It now asks for the document's own URL as `application/pdf`.
- `invoices().email()` always failed: only a `200` was parsed, and the endpoint answers `204` with no body. Any 2xx is parsed now, an empty body as JSON `null`.
- `HistoryRecord::date_utc` was never filled; it now reads Xero's `DateUTCString`.
- PDF and attachment downloads skipped the rate limiter, token refresh and retries, and mapped every failure to `NotFound`.

### Changed
- **Breaking:** `get_attachment` and `get_attachment_by_filename` (accounts, invoices, quotes) take the attachment's `content_type` (its `mime_type`). Xero returns the file only to a request that accepts its type; asked for JSON, it describes the attachment instead.
- `invoices().get_attachment*` take `&self` rather than `&mut self`.

## [0.2.0-alpha.27]

- Add `leave_applications().list_all_v2()` to fetch complete snapshots with duplicate-page detection.
- Apply leave date filters and preserve grouping of custom WHERE clauses.
- Send leave reads through shared token refresh, concurrency and rate-limit handling.
- Add `Client::get_with_modified_since` for payroll URLs and normalize modification timestamps to UTC.
- Complete optional Sentry error matching for server and request-parameter errors.


## [0.2.0-alpha.26]

### Added
- `Client::with_rate_limit(calls_per_minute)` / `without_rate_limit()`: client-side sliding-window pacing for Xero's 60 calls/min per-tenant limit. Reacting to a 429 after the fact does not stop concurrent tasks sharing a tenant from exhausting the budget together, so requests are now paced before they are sent.

### Changed
- Rate-limit retries now add up to 1s of jitter to the `Retry-After` wait. Without it, every task rate limited in the same window woke on the same tick and immediately re-exhausted the limit.

## [0.2.0-alpha.25]

### Added
- Payroll Pay Runs API (`Client::pay_runs()`): `list()`, `get(id)` (with payslip summaries), and `create(payroll_calendar_id)`
- Payroll Payslips API (`Client::payslips()`): `get(id)` (full payslip) and `update_earnings(id, lines)`
- `PayRun`, `PayslipSummary`, `CreatePayRun`, `PayRunResponse` entities (`payroll::pay_run`), with pay-period dates parsed from Xero's `/Date()/` format
- Full `Payslip` entity (`payroll::payslip`) capturing summary totals (wages, deductions, reimbursements, tax, super, net pay) and every line array (earnings, timesheet earnings, leave earnings, deductions, reimbursements, superannuation, tax, leave accrual)
- `EarningsLine::units(rate_id, number_of_units)` helper for fixed-rate earnings updates (e.g. per-km motor vehicle allowance)

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
