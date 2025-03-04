#[macro_use]
extern crate tracing;

use anyhow::Result;
use xero_rs::{oauth::KeyPair, Client, XeroScope};
use std::env;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simpler tracing configuration
    tracing_subscriber::fmt::init();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");

    let scopes = vec![
        XeroScope::accounting_transactions(),
        XeroScope::accounting_settings(),
        XeroScope::accounting_contacts(),
    ];

    info!("Creating client with credentials...");
    let client = match Client::from_client_credentials(KeyPair::new(client_id, Some(client_secret)), Some(scopes)).await {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create client: {:?}", e);
            return Err(e.into());
        }
    };

    info!("Fetching available connections...");
    match xero_rs::connection::list(&client).await {
        Ok(connections) => {
            if connections.is_empty() {
                info!("No connections found");
            } else {
                info!("Found {} connections", connections.len());
                for connection in connections {
                    info!("Tenant ID: {}", connection.tenant_id);
                }
            }
        }
        Err(e) => {
            error!("Failed to list connections: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
} 