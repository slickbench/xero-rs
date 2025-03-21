use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
pub(crate) struct ListResponse {
    pub contacts: Vec<Contact>,
}
