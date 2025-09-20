use anyhow::Result;
use std::env;
use tracing::{error, info};
use xero_rs::{Client, oauth::KeyPair, scope::Scope};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    // Get credentials from environment
    let client_id = env::var("XERO_CLIENT_ID").expect("XERO_CLIENT_ID must be set");
    let client_secret = env::var("XERO_CLIENT_SECRET").expect("XERO_CLIENT_SECRET must be set");

    // Create a Scope using the all accounting helper method
    let scopes = Scope::all_accounting();

    info!("Creating client with credentials...");
    let mut client = match Client::from_client_credentials(
        KeyPair::new(client_id, Some(client_secret)),
        Some(scopes),
    )
    .await
    {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create client: {:?}", e);
            return Err(anyhow::anyhow!("{:?}", e));
        }
    };

    info!("Fetching available connections...");
    match xero_rs::entities::connection::list(&mut client).await {
        Ok(connections) => {
            if connections.is_empty() {
                info!("No connections found");
            } else {
                info!("Found {} connections", connections.len());
                for connection in connections {
                    info!("Tenant ID: {}", connection.tenant_id);
                    info!("Tenant Name: {}", connection.tenant_name);
                    info!("Tenant Type: {}", connection.tenant_type);
                }
            }
        }
        Err(e) => {
            error!("Failed to list connections: {}", e);
            return Err(anyhow::anyhow!("{:?}", e));
        }
    }

    Ok(())
}
