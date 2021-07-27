# xero-rs

[![Crates.io][crates-badge]][crates-url]
[![Documentation][docs-badge]][docs-url]
[![MIT licensed][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]

[crates-badge]: https://img.shields.io/crates/v/xero-rs.svg
[crates-url]: https://crates.io/crates/xero-rs
[docs-badge]: https://docs.rs/xero-rs/badge.svg
[docs-url]: https://docs.rs/xero-rs
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: LICENSE
[actions-badge]: https://github.com/slickbench/xero-rs/actions/workflows/rust.yml/badge.svg
[actions-url]: https://github.com/slickbench/xero-rs/actions/workflows/rust.yml

## Description

A Xero API client library for Rust.

## Functionality

This library is in very early days. This has been implemented so far:

- OAuth2 Authentication (Client Credentials, Authorization Code Flow)
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
