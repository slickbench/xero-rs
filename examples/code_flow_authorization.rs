#[macro_use]
extern crate tracing;

use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;
use serde::Deserialize;
use tokio::sync::Mutex;
use url::Url;
use warp::Filter;
use xero_rs::{Client, oauth::KeyPair};

lazy_static::lazy_static! {
    static ref REDIRECT_ARGS: Arc<Mutex<Option<RedirectArgs>>> = Arc::new(Mutex::new(None));
}

#[derive(Clone, Deserialize)]
struct RedirectArgs {
    code: String,
    state: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct CallbackQuery {
    code: String,
    state: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Spawn a local web server to handle the OAuth callback
    std::thread::spawn(|| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let redirect = warp::get()
                .and(warp::path("redirect"))
                .and(warp::query::<RedirectArgs>())
                .map(|args: RedirectArgs| {
                    tokio::spawn(async move {
                        *REDIRECT_ARGS.lock().await = Some(args);
                    });
                    warp::reply::html("success")
                });
            warp::serve(redirect).run(([127, 0, 0, 1], 4000)).await
        });
    });

    // Create keypair from environment variables
    let key_pair = KeyPair::from_env();
    let redirect_url = Url::from_str("http://localhost:4000/redirect")?;

    // Get authorization URL and CSRF token
    let (authorize_url, csrf_token) = Client::authorize_url(
        key_pair.clone(),
        redirect_url.clone(),
        xero_rs::scope::Scope::accounting_transactions_read(),
    );
    info!("Sign in to Xero: {}", authorize_url.to_string());

    // Wait for the callback with authorization code
    info!("Waiting for redirect URL to be hit...");
    let RedirectArgs { code, state } = loop {
        tokio::time::sleep(Duration::from_millis(10)).await;
        if let Some(args) = REDIRECT_ARGS.try_lock().ok().and_then(|c| c.clone()) {
            break args;
        }
    };
    assert_eq!(&state.expect("missing state"), csrf_token.secret());

    // Exchange authorization code for access token
    let mut client = Client::from_authorization_code(key_pair, redirect_url, code).await?;

    // List available connections
    let connections = xero_rs::entities::connection::list(&mut client).await?;
    info!("found client connections: {:#?}", connections);

    // Select the first tenant and set it on the client
    let tenant_id = connections.first().expect("No connections found").tenant_id;
    client.set_tenant(Some(tenant_id));

    // List invoices
    let invoices = client
        .invoices()
        .list(xero_rs::entities::invoice::ListParameters::default())
        .await?;
    info!("Found {} invoices", invoices.len());

    Ok(())
}
