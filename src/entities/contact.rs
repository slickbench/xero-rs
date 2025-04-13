use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Status {
    Active,
    Archived,
    GdprRequest,
}
/// A contact identifier used for referencing a contact in documents
/// like invoices and quotes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactIdentifier {
    /// Identify the contact by its Xero ID
    ID(Uuid),
    /// Identify the contact by its number
    Number(String),
    /// Identify the contact by its name
    Name(String),
}

impl Serialize for ContactIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            ContactIdentifier::ID(id) => {
                map.serialize_entry("ContactID", id)?;
            }
            ContactIdentifier::Number(number) => {
                map.serialize_entry("ContactNumber", number)?;
            }
            ContactIdentifier::Name(name) => {
                map.serialize_entry("ContactName", name)?;
            }
        }
        map.end()
    }
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
