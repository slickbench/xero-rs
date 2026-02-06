//! Sentry integration for xero-rs errors.
//!
//! This module provides integration with Sentry for error reporting and breadcrumb capture.
//! It is only available when the `sentry` feature is enabled.
//!
//! # Usage
//!
//! Enable the `sentry` feature in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! xero-rs = { version = "0.2", features = ["sentry"] }
//! ```
//!
//! Then set up tracing with `ErrorLayer` and `sentry-tracing`:
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
//! Errors from xero-rs will automatically include span traces when they occur
//! within an instrumented span.

use std::collections::BTreeMap;

use sentry_core::{Breadcrumb, protocol::Value};

use crate::error::{Error, ErrorType, RateLimitType};

/// Convert an xero-rs Error into a Sentry breadcrumb.
///
/// This implementation captures relevant context from xero-rs errors
/// as Sentry breadcrumbs, making it easy to track API call history.
impl<'a> From<&'a Error> for Breadcrumb {
    fn from(error: &'a Error) -> Self {
        let (category, message, data) = match error {
            Error::Request { source, .. } => (
                "http.request",
                format!("HTTP request error: {source}"),
                BTreeMap::new(),
            ),

            Error::DeserializationError {
                entity_type,
                method,
                url,
                status_code,
                ..
            } => {
                let mut data = BTreeMap::new();
                data.insert("entity_type".to_string(), Value::from(entity_type.clone()));
                data.insert("method".to_string(), Value::from(method.clone()));
                data.insert("url".to_string(), Value::from(url.clone()));
                data.insert("status_code".to_string(), Value::from(status_code.clone()));
                (
                    "http.response",
                    format!("Failed to deserialize {entity_type} response"),
                    data,
                )
            }

            Error::NotFound {
                entity,
                url,
                status_code,
                ..
            } => {
                let mut data = BTreeMap::new();
                data.insert("entity".to_string(), Value::from(entity.clone()));
                data.insert("url".to_string(), Value::from(url.clone()));
                data.insert("status_code".to_string(), Value::from(status_code.as_u16()));
                ("http.response", format!("{entity} not found"), data)
            }

            Error::API { response, .. } => {
                let mut data = BTreeMap::new();
                if let Some(error_num) = response.error_number {
                    data.insert("error_number".to_string(), Value::from(error_num));
                }
                if let Some(msg) = &response.message {
                    data.insert("message".to_string(), Value::from(msg.clone()));
                }
                let error_type = match &response.error {
                    ErrorType::ValidationException { .. } => "ValidationException",
                    ErrorType::PostDataInvalidException => "PostDataInvalidException",
                    ErrorType::QueryParseException => "QueryParseException",
                    ErrorType::ObjectNotFoundException => "ObjectNotFoundException",
                    ErrorType::OrganisationOfflineException => "OrganisationOfflineException",
                    ErrorType::UnauthorisedException => "UnauthorisedException",
                    ErrorType::NoDataProcessedException => "NoDataProcessedException",
                    ErrorType::UnsupportedMediaTypeException => "UnsupportedMediaTypeException",
                    ErrorType::MethodNotAllowedException => "MethodNotAllowedException",
                    ErrorType::InternalServerException => "InternalServerException",
                    ErrorType::NotImplementedException => "NotImplementedException",
                    ErrorType::NotAvailableException => "NotAvailableException",
                    ErrorType::RateLimitExceededException => "RateLimitExceededException",
                    ErrorType::SystemUnavailableException => "SystemUnavailableException",
                    ErrorType::Other(s) => s.as_str(),
                };
                data.insert("error_type".to_string(), Value::from(error_type));
                ("xero.api", format!("Xero API error: {error_type}"), data)
            }

            Error::RateLimitExceeded {
                limit_type,
                retry_after,
                url,
                ..
            } => {
                let mut data = BTreeMap::new();
                let limit_str = match limit_type {
                    RateLimitType::Minute => "minute",
                    RateLimitType::Daily => "daily",
                    RateLimitType::AppMinute => "app_minute",
                    RateLimitType::Concurrent => "concurrent",
                    RateLimitType::Unknown(s) => s.as_str(),
                };
                data.insert("limit_type".to_string(), Value::from(limit_str));
                data.insert("url".to_string(), Value::from(url.clone()));
                if let Some(retry) = retry_after {
                    data.insert("retry_after_secs".to_string(), Value::from(retry.as_secs()));
                }
                (
                    "xero.rate_limit",
                    format!("Rate limit exceeded: {limit_str}"),
                    data,
                )
            }

            Error::OAuth2(_) => ("auth", "OAuth2 error".to_string(), BTreeMap::new()),

            Error::Forbidden(_) => (
                "auth",
                "Forbidden - authentication error".to_string(),
                BTreeMap::new(),
            ),

            Error::InvalidEndpoint => (
                "xero.config",
                "Invalid endpoint URL".to_string(),
                BTreeMap::new(),
            ),

            Error::InvalidFilename => (
                "xero.validation",
                "Invalid filename".to_string(),
                BTreeMap::new(),
            ),

            Error::AttachmentTooLarge => (
                "xero.validation",
                "Attachment too large".to_string(),
                BTreeMap::new(),
            ),
        };

        Breadcrumb {
            ty: "error".to_string(),
            category: Some(category.to_string()),
            message: Some(message),
            data,
            level: sentry_core::Level::Error,
            ..Default::default()
        }
    }
}

/// Convert an xero-rs Error into Sentry context data.
///
/// This function extracts relevant information from an error
/// for use as additional Sentry context.
///
/// # Example
///
/// ```ignore
/// use sentry::configure_scope;
/// use xero_rs::sentry_integration::error_to_sentry_context;
///
/// if let Err(e) = client.contacts().list().await {
///     configure_scope(|scope| {
///         let context = error_to_sentry_context(&e);
///         for (key, value) in context {
///             scope.set_extra(&key, value);
///         }
///     });
/// }
/// ```
pub fn error_to_sentry_context(error: &Error) -> BTreeMap<String, Value> {
    let mut context = BTreeMap::new();

    // Add span trace if available
    if let Some(span_trace) = error.span_trace() {
        context.insert(
            "xero.span_trace".to_string(),
            Value::from(format!("{span_trace}")),
        );
    }

    // Add URL if available
    if let Some(url) = error.url() {
        context.insert("xero.url".to_string(), Value::from(url.to_string()));
    }

    // Add status code if available
    if let Some(status) = error.status_code() {
        context.insert("xero.status_code".to_string(), Value::from(status.as_u16()));
    }

    // Add response body preview if available
    if let Some(body) = error.response_body() {
        // Truncate for Sentry
        let truncated = if body.len() > 500 {
            format!("{}...", &body[..500])
        } else {
            body.to_string()
        };
        context.insert("xero.response_body".to_string(), Value::from(truncated));
    }

    // Add API response details if available
    if let Some(response) = error.api_response() {
        if let Some(error_num) = response.error_number {
            context.insert("xero.error_number".to_string(), Value::from(error_num));
        }
        if let Some(msg) = &response.message {
            context.insert("xero.message".to_string(), Value::from(msg.clone()));
        }
    }

    context
}
