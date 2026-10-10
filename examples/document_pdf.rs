//! Download the PDF Xero renders for a purchase order, invoice or quote.
//!
//! ```sh
//! cargo run --example document_pdf -- purchase-order [ID] [OUT.pdf]
//! cargo run --example document_pdf -- invoice [ID] [OUT.pdf]
//! cargo run --example document_pdf -- quote [ID] [OUT.pdf]
//! ```
//!
//! Without an id, the first one Xero lists is used. Read only: nothing is changed in Xero.

#[macro_use]
extern crate tracing;

use anyhow::{Context, Result, bail};
use uuid::Uuid;
use xero_rs::{
    Client,
    oauth::KeyPair,
    scope::{Permission, Scope, ScopeType},
};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut args = std::env::args().skip(1);
    let kind = args.next().unwrap_or_else(|| "purchase-order".to_string());
    let id = args.next().map(|id| Uuid::parse_str(&id)).transpose()?;
    let out = args.next();

    let client_id = std::env::var("XERO_CLIENT_ID").context("XERO_CLIENT_ID must be set")?;
    let client_secret =
        std::env::var("XERO_CLIENT_SECRET").context("XERO_CLIENT_SECRET must be set")?;
    // A custom connection is refused a scope it was not authorised with, and that includes the
    // read-only variant of a read-write scope it was - so ask for read-write, and only read.
    let client = Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        Some(Scope::from(vec![ScopeType::AccountingTransactions(
            Permission::ReadWrite,
        )])),
    )
    .await?;

    let (id, pdf) = match kind.as_str() {
        "purchase-order" => {
            let api = client.purchase_orders();
            let id = match id {
                Some(id) => id,
                None => {
                    api.list()
                        .await?
                        .first()
                        .context("no purchase orders")?
                        .purchase_order_id
                }
            };
            (id, api.get_pdf(id).await?)
        }
        "invoice" => {
            let api = client.invoices();
            let id = match id {
                Some(id) => id,
                None => {
                    api.list_all()
                        .await?
                        .first()
                        .context("no invoices")?
                        .invoice_id
                }
            };
            (id, api.get_pdf(id).await?)
        }
        "quote" => {
            let api = client.quotes();
            let id = match id {
                Some(id) => id,
                None => api.list_all().await?.first().context("no quotes")?.quote_id,
            };
            (id, api.get_pdf(id).await?)
        }
        other => bail!("unknown document kind {other}: purchase-order, invoice or quote"),
    };

    if !pdf.starts_with(b"%PDF-") {
        bail!("Xero answered {} bytes that are not a PDF", pdf.len());
    }
    let out = out.unwrap_or_else(|| format!("{kind}-{id}.pdf"));
    std::fs::write(&out, &pdf)?;
    info!(bytes = pdf.len(), path = %out, "Saved the PDF");

    Ok(())
}
