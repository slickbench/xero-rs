#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]

#[macro_use]
extern crate tracing;

pub mod client;
pub mod connection;
pub mod error;
pub mod oauth;

pub use client::Client;
pub use oauth::KeyPair;
