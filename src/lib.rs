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

pub use error::Error;
pub use client::Client;
pub use endpoints::XeroEndpoint;
pub use entities::*;
pub use oauth::KeyPair;
pub use scope::{Permission, Scope, ScopeCategory, ScopeType};
