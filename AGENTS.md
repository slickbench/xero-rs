# Repository Guidelines

## Project Structure & Module Organization
- `src/` hosts crate entry points; keep new modules snake_case and re-export via the relevant `mod.rs`.
- Domain types live in `src/entities/` and payroll logic in `src/payroll/`; consult `Xero-OpenAPI/` when modeling and share helpers through `src/utils/`.
- Integration tests in `tests/` mirror Xero areas (`invoice.rs`, `timesheet.rs`, etc.); add new scenarios beside peers and reuse `tests/test_utils.rs`.
- `examples/` covers runnable flows; update or add one whenever a feature needs manual validation.

## Build, Test, and Development Commands
- `cargo check` runs a fast compile guard.
- `cargo fmt` applies Rustfmt so CI hooks stay green.
- `cargo clippy --all-features` runs the lint suite used in GitHub Actions.
- `cargo test -- --nocapture` runs async integration tests; export Xero secrets first.
- `cargo llvm-cov --all-features --report-html` reproduces the coverage artifact uploaded in CI.

## Coding Style & Naming Conventions
- Follow Rust 2024 defaults: 4-space indentation, Rustfmt ordering, minimal wildcards.
- Apply `PascalCase` to types and traits, keep modules/functions in `snake_case`, and reserve `SCREAMING_SNAKE_CASE` for constants and feature flags.
- Return `miette::Result` from fallible API calls and surface actionable context; avoid panics outside tests.
- Run `pre-commit run --all-files` (fmt, clippy, trailing-whitespace, commitizen) before pushing.

## Testing Guidelines
- Use Tokio’s async harness; name cases `<resource>_<scenario>` for quick filtering (for example `invoice_create_happy_path`).
- Populate `XERO_CLIENT_ID`, `XERO_CLIENT_SECRET`, and `XERO_TENANT_ID` via `.envrc.local` or your shell, never in tracked files.
- Reuse `tests/test_utils.rs` helpers when instantiating clients or scopes to keep credentials handling consistent.
- Check coverage locally with `cargo llvm-cov`; avoid PRs that drop the main-branch trend.

## Commit & Pull Request Guidelines
- Follow Conventional Commits (`feat: add purchase order filters`, `fix: handle relative URLs`) as enforced by the Commitizen hook.
- Group formatting-only updates where practical and include rerun output for the commands above in the PR body.
- PR descriptions should explain behavior changes, list new secrets or config, and call out manual verification steps.
- Link related issues, request at least one reviewer, and attach screenshots or API traces for user-visible or request payload changes.

## Security & Configuration Tips
- Keep API credentials out of the repo; rely on `direnv allow` or a password manager to load secrets ad hoc.
- Rotate sandbox credentials regularly and note upcoming expirations in the PR discussion when they matter.
- Run `rate_limit_test.sh` only against non-production tenants to avoid triggering global throttles.
