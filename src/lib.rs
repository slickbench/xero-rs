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

pub use client::Client;
pub use endpoints::XeroEndpoint;
pub use entities::*;
pub use error::Error;
pub use oauth::KeyPair;
pub use scope::{Permission, Scope, ScopeCategory, ScopeType};

// Re-export LineAmountType
pub use entities::line_item::LineAmountType as line_amount_types;

// Re-export Item types for convenience
pub use entities::item::{Item, PurchaseDetails, SalesDetails};
