use std::str::FromStr;

use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

use crate::{
    error::{Error, Result},
    Client,
};

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/Contacts";

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Active,
    Archived,
    GdprRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Contact {
    #[serde(rename = "ContactID")]
    pub contact_id: Uuid,
    pub contact_number: Option<String>,
    pub account_number: Option<String>,
    pub contact_status: Option<Status>,
    pub name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email_address: Option<String>,
    pub skype_user_name: Option<String>,
    pub bank_account_details: Option<String>,
    pub tax_number: Option<String>,
    /*pub accounts_receivable_tax_type: TaxType,
    pub accounts_payable_tax_type: TaxType,
    pub addresses: Vec<Address>,
    pub phones: Vec<Phone>,*/
    pub is_supplier: Option<bool>,
    pub is_customer: Option<bool>,
    pub default_currency: Option<String>,
    #[serde(rename = "UpdatedDateUTC")]
    pub updated_date_utc: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListResponse {
    contacts: Vec<Contact>,
}

/// Retrieve a list of contacts.
#[instrument(skip(client))]
pub async fn list(client: &Client) -> Result<Vec<Contact>> {
    let response: ListResponse = client.get(ENDPOINT, Vec::<String>::default()).await?;
    Ok(response.contacts)
}

/// Retrieve a single contact by it's `contact_id`.
#[instrument(skip(client))]
pub async fn get(client: &Client, contact_id: Uuid) -> Result<Contact> {
    let endpoint = Url::from_str(ENDPOINT)
        .and_then(|endpoint| endpoint.join(&contact_id.to_string()))
        .map_err(|_| Error::InvalidEndpoint)?;
    let endpoint_str = endpoint.to_string();
    let response: ListResponse = client.get(endpoint, Vec::<String>::default()).await?;
    response.contacts.into_iter().next().ok_or(Error::NotFound {
        entity: "Contact".to_string(),
        url: endpoint_str,
        status_code: reqwest::StatusCode::NOT_FOUND,
        response_body: Some(format!("Contact with ID {contact_id} not found")),
    })
}
