use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    Client,
    endpoints::XeroEndpoint,
    entities::{EntityEndpoint, MutationResponse, endpoint_utils},
    error::{Error, Result},
    utils::{date_format::xero_datetime_format, serde_helpers::empty_string_as_none},
};

pub const ENDPOINT: &str = "https://api.xero.com/api.xro/2.0/Accounts/";

/// Account types in Xero
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AccountType {
    Bank,
    Current,
    #[serde(rename = "CURRLIAB")]
    CurrentLiability,
    #[serde(rename = "DEPRECIATN")]
    Depreciation,
    #[serde(rename = "DIRECTCOSTS")]
    DirectCosts,
    Equity,
    Expense,
    Fixed,
    Inventory,
    Liability,
    #[serde(rename = "NONCURRENT")]
    NonCurrent,
    #[serde(rename = "OTHERINCOME")]
    OtherIncome,
    Overheads,
    Prepayment,
    Revenue,
    Sales,
    #[serde(rename = "TERMLIAB")]
    TermLiability,
    #[serde(rename = "PAYG")]
    Payg,
}

/// Account status codes
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AccountStatus {
    Active,
    Archived,
    Deleted,
}

/// Account class types (read-only)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum AccountClass {
    Asset,
    Equity,
    Expense,
    Liability,
    Revenue,
}

/// Bank account types
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum BankAccountType {
    Bank,
    #[serde(rename = "CREDITCARD")]
    CreditCard,
    #[serde(rename = "PAYPAL")]
    PayPal,
}

/// Represents an account in the chart of accounts
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Account {
    /// Unique identifier for the account
    #[serde(rename = "AccountID")]
    pub account_id: Uuid,

    /// Customer defined alpha numeric account code (max 10 chars)
    /// Note: System accounts may not have a code
    #[serde(default)]
    pub code: Option<String>,

    /// Name of the account (max 150 chars)
    #[serde(default)]
    pub name: String,

    /// Account type
    #[serde(rename = "Type", default)]
    pub account_type: Option<AccountType>,

    /// Account status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AccountStatus>,

    /// Description of the account (max 4000 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Tax type from TaxRates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,

    /// Account class (read-only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<AccountClass>,

    /// System account type (read-only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_account: Option<String>,

    /// Whether payments can be applied to this account
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_payments_to_account: Option<bool>,

    /// Whether account is available for expense claims
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_in_expense_claims: Option<bool>,

    /// Bank account number (for BANK type accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_account_number: Option<String>,

    /// Bank account type (for BANK type accounts only - non-bank accounts return "")
    #[serde(
        default,
        deserialize_with = "empty_string_as_none",
        skip_serializing_if = "Option::is_none"
    )]
    pub bank_account_type: Option<BankAccountType>,

    /// Currency code for the account
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_code: Option<String>,

    /// Reporting code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_code: Option<String>,

    /// Reporting code name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reporting_code_name: Option<String>,

    /// Whether the account has attachments
    #[serde(default)]
    pub has_attachments: bool,

    /// Last modified date
    #[serde(rename = "UpdatedDateUTC", with = "xero_datetime_format")]
    pub updated_date_utc: OffsetDateTime,

    /// Validation errors from the API
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_errors: Vec<ValidationError>,
}

/// Validation error returned by the API
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ValidationError {
    pub message: String,
}

/// Response wrapper for listing accounts
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ListResponse {
    pub accounts: Vec<Account>,
}

impl From<ListResponse> for Vec<Account> {
    fn from(response: ListResponse) -> Self {
        response.accounts
    }
}

/// Parameters for listing accounts
#[derive(Debug, Serialize, Default)]
pub struct ListParameters {
    /// Filter by any element
    #[serde(rename = "where", skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,

    /// Order by any element
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>,
}

impl ListParameters {
    /// Create a new builder for `ListParameters`
    #[must_use]
    pub fn builder() -> Self {
        Self::default()
    }

    /// Set the where filter
    #[must_use]
    pub fn with_where(mut self, filter: impl Into<String>) -> Self {
        self.r#where = Some(filter.into());
        self
    }

    /// Set the order clause
    #[must_use]
    pub fn with_order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }

    /// Filter by account type
    #[must_use]
    pub fn with_type(self, account_type: AccountType) -> Self {
        let type_str = match account_type {
            AccountType::Bank => "BANK",
            AccountType::Current => "CURRENT",
            AccountType::CurrentLiability => "CURRLIAB",
            AccountType::Depreciation => "DEPRECIATN",
            AccountType::DirectCosts => "DIRECTCOSTS",
            AccountType::Equity => "EQUITY",
            AccountType::Expense => "EXPENSE",
            AccountType::Fixed => "FIXED",
            AccountType::Inventory => "INVENTORY",
            AccountType::Liability => "LIABILITY",
            AccountType::NonCurrent => "NONCURRENT",
            AccountType::OtherIncome => "OTHERINCOME",
            AccountType::Overheads => "OVERHEADS",
            AccountType::Prepayment => "PREPAYMENT",
            AccountType::Revenue => "REVENUE",
            AccountType::Sales => "SALES",
            AccountType::TermLiability => "TERMLIAB",
            AccountType::Payg => "PAYG",
        };
        self.with_where(format!("Type==\"{}\"", type_str))
    }

    /// Filter by account status
    #[must_use]
    pub fn with_status(self, status: AccountStatus) -> Self {
        let status_str = match status {
            AccountStatus::Active => "ACTIVE",
            AccountStatus::Archived => "ARCHIVED",
            AccountStatus::Deleted => "DELETED",
        };
        self.with_where(format!("Status==\"{}\"", status_str))
    }

    /// Filter by account class
    #[must_use]
    pub fn with_class(self, class: AccountClass) -> Self {
        let class_str = match class {
            AccountClass::Asset => "ASSET",
            AccountClass::Equity => "EQUITY",
            AccountClass::Expense => "EXPENSE",
            AccountClass::Liability => "LIABILITY",
            AccountClass::Revenue => "REVENUE",
        };
        self.with_where(format!("Class==\"{}\"", class_str))
    }
}

/// Builder for creating or updating accounts
#[derive(Debug, Serialize, Clone, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Builder {
    /// Customer defined alpha numeric account code (max 10 chars)
    pub code: String,

    /// Name of the account (max 150 chars)
    pub name: String,

    /// Account type
    #[serde(rename = "Type")]
    pub account_type: Option<AccountType>,

    /// Account status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<AccountStatus>,

    /// Description of the account (max 4000 chars)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Tax type from TaxRates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_type: Option<String>,

    /// Whether payments can be applied to this account
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_payments_to_account: Option<bool>,

    /// Whether account is available for expense claims
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_in_expense_claims: Option<bool>,

    /// Bank account number (for BANK type accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_account_number: Option<String>,

    /// Bank account type (for BANK type accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bank_account_type: Option<BankAccountType>,

    /// Currency code for the account
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency_code: Option<String>,

    /// Account ID (for updates)
    #[serde(rename = "AccountID", skip_serializing_if = "Option::is_none")]
    pub account_id: Option<Uuid>,
}

impl Builder {
    /// Create a new account builder
    #[must_use]
    pub fn new(
        code: impl Into<String>,
        name: impl Into<String>,
        account_type: AccountType,
    ) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            account_type: Some(account_type),
            ..Default::default()
        }
    }

    /// Set the status
    #[must_use]
    pub fn with_status(mut self, status: AccountStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the description
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the tax type
    #[must_use]
    pub fn with_tax_type(mut self, tax_type: impl Into<String>) -> Self {
        self.tax_type = Some(tax_type.into());
        self
    }

    /// Set whether payments can be applied to this account
    #[must_use]
    pub fn with_enable_payments_to_account(mut self, enable: bool) -> Self {
        self.enable_payments_to_account = Some(enable);
        self
    }

    /// Set whether account is available for expense claims
    #[must_use]
    pub fn with_show_in_expense_claims(mut self, show: bool) -> Self {
        self.show_in_expense_claims = Some(show);
        self
    }

    /// Set the bank account number (for BANK type accounts)
    #[must_use]
    pub fn with_bank_account_number(mut self, number: impl Into<String>) -> Self {
        self.bank_account_number = Some(number.into());
        self
    }

    /// Set the bank account type (for BANK type accounts)
    #[must_use]
    pub fn with_bank_account_type(mut self, bank_type: BankAccountType) -> Self {
        self.bank_account_type = Some(bank_type);
        self
    }

    /// Set the currency code
    #[must_use]
    pub fn with_currency_code(mut self, currency: impl Into<String>) -> Self {
        self.currency_code = Some(currency.into());
        self
    }

    /// Set the account ID (for updates)
    #[must_use]
    pub fn with_account_id(mut self, id: Uuid) -> Self {
        self.account_id = Some(id);
        self
    }
}

/// Request wrapper for accounts
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct AccountWrapper<'a> {
    pub accounts: Vec<&'a Builder>,
}

/// Attachment details for an account
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Attachment {
    #[serde(rename = "AttachmentID")]
    pub attachment_id: Uuid,
    pub file_name: String,
    pub url: String,
    pub mime_type: String,
    pub content_length: i64,
}

/// Attachments response wrapper
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Attachments {
    pub attachments: Vec<Attachment>,
}

impl EntityEndpoint<Account, ListParameters> for Account {
    fn endpoint() -> &'static str {
        ENDPOINT
    }

    async fn get(client: &Client, id: Uuid) -> Result<Account> {
        endpoint_utils::get::<Account, ListResponse>(client, ENDPOINT, id, "Account").await
    }

    async fn list(client: &Client, params: ListParameters) -> Result<Vec<Account>> {
        endpoint_utils::list::<Account, ListResponse, ListParameters>(client, ENDPOINT, &params)
            .await
    }
}

/// List accounts with optional parameters
pub async fn list(client: &Client, params: ListParameters) -> Result<Vec<Account>> {
    Account::list(client, params).await
}

/// List all accounts without any filtering
pub async fn list_all(client: &Client) -> Result<Vec<Account>> {
    Account::list(client, ListParameters::default()).await
}

/// Get a single account by ID
pub async fn get(client: &Client, account_id: Uuid) -> Result<Account> {
    Account::get(client, account_id).await
}

/// Create a new account
pub async fn create(client: &Client, account: &Builder) -> Result<Account> {
    let wrapper = AccountWrapper {
        accounts: vec![account],
    };

    let response: MutationResponse = client
        .put_endpoint(XeroEndpoint::Accounts, &wrapper)
        .await?;

    response
        .data
        .get_accounts()
        .and_then(|accounts| accounts.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Account".to_string(),
            url: ENDPOINT.to_string(),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some("No account returned in response".to_string()),
        })
}

/// Update an existing account
pub async fn update(client: &Client, account_id: Uuid, account: &Builder) -> Result<Account> {
    let mut account_with_id = account.clone();
    account_with_id.account_id = Some(account_id);

    let wrapper = AccountWrapper {
        accounts: vec![&account_with_id],
    };

    let endpoint = XeroEndpoint::Account(account_id);
    let response: MutationResponse = client.post_endpoint(endpoint, &wrapper).await?;

    response
        .data
        .get_accounts()
        .and_then(|accounts| accounts.into_iter().next())
        .ok_or(Error::NotFound {
            entity: "Account".to_string(),
            url: format!("{ENDPOINT}{account_id}"),
            status_code: reqwest::StatusCode::NOT_FOUND,
            response_body: Some("No account returned in response".to_string()),
        })
}

/// Delete an account
pub async fn delete(client: &Client, account_id: Uuid) -> Result<()> {
    let endpoint = XeroEndpoint::Account(account_id);
    client.delete_endpoint(endpoint).await
}

/// List all attachments for an account
pub async fn list_attachments(client: &Client, account_id: Uuid) -> Result<Vec<Attachment>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Accounts".to_string(),
        account_id.to_string(),
        "Attachments".to_string(),
    ]);

    let response: Attachments = client.get_endpoint(endpoint, &()).await?;
    Ok(response.attachments)
}

/// Get a specific attachment by ID
pub async fn get_attachment(
    client: &Client,
    account_id: Uuid,
    attachment_id: Uuid,
) -> Result<Vec<u8>> {
    let endpoint = XeroEndpoint::Custom(vec![
        "Accounts".to_string(),
        account_id.to_string(),
        "Attachments".to_string(),
        attachment_id.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::GET, url)
        .await
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        Ok(response.bytes().await?.to_vec())
    } else {
        Err(Error::NotFound {
            entity: "Account Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to retrieve attachment for account with ID {account_id}"
            )),
        })
    }
}

/// Upload an attachment to an account
pub async fn upload_attachment(
    client: &Client,
    account_id: Uuid,
    filename: &str,
    attachment_content: &[u8],
) -> Result<Attachment> {
    use std::ffi::OsStr;
    use std::path::Path;

    const MAX_ATTACHMENT_SIZE: usize = 25 * 1024 * 1024; // 25 MB

    if filename.is_empty() {
        return Err(Error::InvalidFilename);
    }

    let ext = Path::new(filename).extension().and_then(OsStr::to_str);
    let content_type = match ext {
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("txt") => "text/plain",
        Some("csv") => "text/csv",
        _ => "application/octet-stream",
    };

    if attachment_content.len() > MAX_ATTACHMENT_SIZE {
        return Err(Error::AttachmentTooLarge);
    }

    let endpoint = XeroEndpoint::Custom(vec![
        "Accounts".to_string(),
        account_id.to_string(),
        "Attachments".to_string(),
        filename.to_string(),
    ]);

    let url = endpoint.to_url()?;
    let response = client
        .build_request(reqwest::Method::PUT, url)
        .await
        .header(reqwest::header::CONTENT_TYPE, content_type)
        .header(reqwest::header::CONTENT_LENGTH, attachment_content.len())
        .body(attachment_content.to_vec())
        .send()
        .await?;

    let status = response.status();

    if status.is_success() {
        let attachments: Attachments = response.json().await?;
        attachments
            .attachments
            .into_iter()
            .next()
            .ok_or(Error::NotFound {
                entity: "Account Attachment".to_string(),
                url: endpoint.to_string(),
                status_code: status,
                response_body: Some("No attachment was returned after upload".to_string()),
            })
    } else {
        Err(Error::NotFound {
            entity: "Account Attachment".to_string(),
            url: endpoint.to_string(),
            status_code: status,
            response_body: Some(format!(
                "Failed to upload attachment for account with ID {account_id}"
            )),
        })
    }
}
