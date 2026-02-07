//! # xero-rs
//!
//! A Rust client library for the Xero API.
//!
//! ## Sentry Integration
//!
//! This library provides rich error context that integrates well with Sentry
//! for error monitoring. Errors include async span traces that capture the
//! call stack at the point of error creation.
//!
//! ### Setup with Sentry
//!
//! To enable full Sentry integration with span traces:
//!
//! 1. Enable the `sentry` feature (optional, for `IntoBreadcrumb` support):
//!
//! ```toml
//! [dependencies]
//! xero-rs = { version = "0.2", features = ["sentry"] }
//! ```
//!
//! 2. Set up tracing with `ErrorLayer` and `sentry-tracing`:
//!
//! ```ignore
//! use tracing_subscriber::prelude::*;
//! use tracing_error::ErrorLayer;
//!
//! tracing_subscriber::registry()
//!     .with(tracing_subscriber::fmt::layer())
//!     .with(ErrorLayer::default())  // Required for SpanTrace capture
//!     .with(sentry::integrations::tracing::layer())
//!     .init();
//! ```
//!
//! 3. Errors will now automatically include span traces when reported to Sentry:
//!
//! ```ignore
//! if let Err(e) = client.contacts().list().await {
//!     // The error includes a span trace
//!     if let Some(trace) = e.span_trace() {
//!         eprintln!("Span trace:\n{}", trace);
//!     }
//!
//!     // Report to Sentry - the span trace provides async call context
//!     sentry::capture_error(&e);
//! }
//! ```

#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

#[macro_use]
extern crate tracing;

pub mod client;
pub mod endpoints;
pub mod entities;
pub mod error;
pub mod oauth;
pub mod payroll;
pub mod scope;
pub mod utils;

#[cfg(feature = "sentry")]
pub mod sentry_integration;

pub use client::Client;
pub use endpoints::XeroEndpoint;
pub use entities::*;
pub use error::{Error, RateLimitType};
pub use oauth::KeyPair;
pub use scope::{Permission, Scope, ScopeCategory, ScopeType};

/// Unit decimal places for line item amounts.
/// Controls precision of `UnitAmount` values in the Xero API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitDp {
    /// 2 decimal places (Xero default)
    Two,
    /// 4 decimal places (higher precision)
    Four,
}

impl UnitDp {
    pub(crate) fn as_u8(self) -> u8 {
        match self {
            Self::Two => 2,
            Self::Four => 4,
        }
    }
}

impl serde::Serialize for UnitDp {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u8(self.as_u8())
    }
}

/// Options for mutation (create/update) API requests.
/// These control query parameters appended to PUT/POST URLs.
#[derive(Debug, Default, Clone)]
pub(crate) struct MutationOptions {
    /// Unit decimal places for line item amounts.
    pub unitdp: Option<UnitDp>,
}

impl MutationOptions {
    /// Apply the options as query parameters to a URL.
    pub fn apply_to_url(&self, url: &mut url::Url) {
        if let Some(unitdp) = self.unitdp {
            url.query_pairs_mut()
                .append_pair("unitdp", &unitdp.as_u8().to_string());
        }
    }
}

/// Query struct for passing unitdp on GET requests.
#[derive(Debug, serde::Serialize, Default)]
pub(crate) struct UnitDpQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unitdp: Option<UnitDp>,
}

// Re-export SpanTrace for users who want to access it
pub use tracing_error::SpanTrace;

// Re-export LineAmountType
pub use entities::line_item::LineAmountType as line_amount_types;

// Re-export Account types for convenience
pub use entities::account::{Account, AccountClass, AccountStatus, AccountType, BankAccountType};

// Re-export Item types for convenience
pub use entities::item::{Item, PurchaseDetails, SalesDetails};

// Re-export Leave Application types for convenience
pub use payroll::leave_application::{
    LeaveApplication, LeavePeriod, LeavePeriodStatus,
    ListParameters as LeaveApplicationListParameters, PayOutType, PostLeaveApplication,
};

// Re-export Leave Type for convenience
pub use payroll::settings::leave_types::LeaveType;
