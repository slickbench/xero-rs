//! What Xero keeps alike on its documents: the history and notes, the PDF it renders, and the
//! files attached.
//!
//! Invoices, quotes, purchase orders, items and accounts lay these out the same way -
//! `{Resource}/{id}/History`, `{Resource}/{id}/Attachments/{name}`, and the document itself asked
//! for as a PDF - so each entity module names its resource and calls through here.

use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::{Client, endpoints::XeroEndpoint, error::Result};

/// The media type Xero renders a document's PDF for.
pub const APPLICATION_PDF: &str = "application/pdf";

/// One entry in a document's history: a change Xero recorded, or a note someone added.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecord {
    /// The details of the history record
    pub details: String,

    /// When it happened, in UTC, as `2026-10-09T06:25:43`. Xero names it `DateUTCString` (it also
    /// sends `DateUTC` in its `/Date(ms)/` form), which `PascalCase` alone never matched.
    #[serde(rename = "DateUTCString", skip_serializing_if = "Option::is_none")]
    pub date_utc: Option<String>,

    /// The user who created the history record
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// The changes made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<String>,
}

/// Wrapper for history records response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecords {
    pub history_records: Vec<HistoryRecord>,
}

/// Wrapper for posting history records
#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct HistoryRecordsRequest {
    pub history_records: Vec<HistoryRecord>,
}

fn path(resource: &str, id: Uuid, tail: &[&str]) -> XeroEndpoint {
    let mut segments = vec![resource.to_string(), id.to_string()];
    segments.extend(tail.iter().map(|segment| (*segment).to_string()));
    XeroEndpoint::Custom(segments)
}

/// Retrieve a document's history.
#[instrument(skip(client))]
pub(crate) async fn get_history(
    client: &Client,
    resource: &str,
    id: Uuid,
) -> Result<Vec<HistoryRecord>> {
    let response: HistoryRecords = client
        .get_endpoint(path(resource, id, &["History"]), &())
        .await?;
    Ok(response.history_records)
}

/// Add a note to a document's history.
#[instrument(skip(client))]
pub(crate) async fn create_history(
    client: &Client,
    resource: &str,
    id: Uuid,
    details: &str,
) -> Result<Vec<HistoryRecord>> {
    let request = HistoryRecordsRequest {
        history_records: vec![HistoryRecord {
            details: details.to_string(),
            date_utc: None,
            user: None,
            changes: None,
        }],
    };
    let response: HistoryRecords = client
        .put_endpoint(path(resource, id, &["History"]), &request)
        .await?;
    Ok(response.history_records)
}

/// Retrieve the PDF Xero renders for a document, in the organisation's branding theme.
///
/// The PDF is the document's own URL asked for as `application/pdf`. Xero's `OpenAPI` spec and SDKs
/// name a `{Resource}/{id}/pdf` path instead, and the live API answers that with a 404 - for
/// invoices, quotes and purchase orders alike (checked 2026-10-10).
#[instrument(skip(client))]
pub(crate) async fn get_pdf(client: &Client, resource: &str, id: Uuid) -> Result<Vec<u8>> {
    client
        .get_bytes(path(resource, id, &[]), APPLICATION_PDF)
        .await
}

/// Retrieve the content of a file attached to a document, by attachment id or file name.
///
/// `content_type` is the attachment's `MimeType`: Xero answers the file itself only to a request
/// that accepts that type, and the attachment's JSON description to anything else.
#[instrument(skip(client))]
pub(crate) async fn get_attachment(
    client: &Client,
    resource: &str,
    id: Uuid,
    attachment: &str,
    content_type: &str,
) -> Result<Vec<u8>> {
    client
        .get_bytes(
            path(resource, id, &["Attachments", attachment]),
            content_type,
        )
        .await
}

#[cfg(test)]
mod tests {
    use super::HistoryRecords;

    #[test]
    fn history_reads_what_xero_sends() {
        // A purchase order's history as the API answered it on 2026-10-10.
        let body = r#"{
            "HistoryRecords": [{
                "Changes": "Edited",
                "DateUTCString": "2026-10-09T06:25:43",
                "DateUTC": "\\/Date(1791527143570+0000)\\/",
                "User": "System Generated",
                "Details": "PO-2610-YGQV Total changed from 765.60 to 1105.50."
            }]
        }"#;

        let records: HistoryRecords = serde_json::from_str(body).unwrap();
        let record = &records.history_records[0];

        assert_eq!(record.date_utc.as_deref(), Some("2026-10-09T06:25:43"));
        assert_eq!(record.user.as_deref(), Some("System Generated"));
        assert_eq!(record.changes.as_deref(), Some("Edited"));
    }
}
