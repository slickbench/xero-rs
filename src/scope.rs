use oauth2::Scope as OAuth2Scope;
use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

/// Represents a category of Xero API scopes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeCategory {
    Accounting,
    Assets,
    Files,
    Payroll,
    Projects,
    Authentication,
}

/// Permission level for a scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ReadWrite,
    ReadOnly,
}

/// Available Xero API scopes
///
/// Each scope represents an area of functionality in the Xero API
/// that can be requested during authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScopeType {
    // Accounting scopes
    AccountingTransactions(Permission),
    AccountingReports,
    AccountingReportsTenninetynine,
    AccountingBudgets,
    AccountingJournals,
    AccountingSettings(Permission),
    AccountingContacts(Permission),
    AccountingAttachments(Permission),

    // Assets scopes
    Assets(Permission),

    // Files scopes
    Files(Permission),

    // Payroll scopes
    PayrollEmployees(Permission),
    PayrollPayruns(Permission),
    PayrollPayslip(Permission),
    PayrollSettings(Permission),
    PayrollTimesheets(Permission),

    // Projects scopes
    Projects(Permission),

    // Offline access scope
    OfflineAccess,
}

impl ScopeType {
    /// Convert a `ScopeType` to its string representation
    fn to_string(&self) -> String {
        match self {
            // Accounting scopes
            Self::AccountingTransactions(Permission::ReadWrite) => "accounting.transactions",
            Self::AccountingTransactions(Permission::ReadOnly) => "accounting.transactions.read",
            Self::AccountingReports => "accounting.reports.read",
            Self::AccountingReportsTenninetynine => "accounting.reports.tenninetynine.read",
            Self::AccountingBudgets => "accounting.budgets.read",
            Self::AccountingJournals => "accounting.journals.read",
            Self::AccountingSettings(Permission::ReadWrite) => "accounting.settings",
            Self::AccountingSettings(Permission::ReadOnly) => "accounting.settings.read",
            Self::AccountingContacts(Permission::ReadWrite) => "accounting.contacts",
            Self::AccountingContacts(Permission::ReadOnly) => "accounting.contacts.read",
            Self::AccountingAttachments(Permission::ReadWrite) => "accounting.attachments",
            Self::AccountingAttachments(Permission::ReadOnly) => "accounting.attachments.read",

            // Assets scopes
            Self::Assets(Permission::ReadWrite) => "assets",
            Self::Assets(Permission::ReadOnly) => "assets.read",

            // Files scopes
            Self::Files(Permission::ReadWrite) => "files",
            Self::Files(Permission::ReadOnly) => "files.read",

            // Payroll scopes
            Self::PayrollEmployees(Permission::ReadWrite) => "payroll.employees",
            Self::PayrollEmployees(Permission::ReadOnly) => "payroll.employees.read",
            Self::PayrollPayruns(Permission::ReadWrite) => "payroll.payruns",
            Self::PayrollPayruns(Permission::ReadOnly) => "payroll.payruns.read",
            Self::PayrollPayslip(Permission::ReadWrite) => "payroll.payslip",
            Self::PayrollPayslip(Permission::ReadOnly) => "payroll.payslip.read",
            Self::PayrollSettings(Permission::ReadWrite) => "payroll.settings",
            Self::PayrollSettings(Permission::ReadOnly) => "payroll.settings.read",
            Self::PayrollTimesheets(Permission::ReadWrite) => "payroll.timesheets",
            Self::PayrollTimesheets(Permission::ReadOnly) => "payroll.timesheets.read",

            // Projects scopes
            Self::Projects(Permission::ReadWrite) => "projects",
            Self::Projects(Permission::ReadOnly) => "projects.read",

            // Offline access scope
            Self::OfflineAccess => "offline_access",
        }
        .to_string()
    }

    /// Get the category of this scope
    #[must_use]
    pub fn category(&self) -> ScopeCategory {
        match self {
            Self::AccountingTransactions(_)
            | Self::AccountingReports
            | Self::AccountingReportsTenninetynine
            | Self::AccountingBudgets
            | Self::AccountingJournals
            | Self::AccountingSettings(_)
            | Self::AccountingContacts(_)
            | Self::AccountingAttachments(_) => ScopeCategory::Accounting,

            Self::Assets(_) => ScopeCategory::Assets,
            Self::Files(_) => ScopeCategory::Files,

            Self::PayrollEmployees(_)
            | Self::PayrollPayruns(_)
            | Self::PayrollPayslip(_)
            | Self::PayrollSettings(_)
            | Self::PayrollTimesheets(_) => ScopeCategory::Payroll,

            Self::Projects(_) => ScopeCategory::Projects,

            Self::OfflineAccess => ScopeCategory::Authentication,
        }
    }
}

/// Error when parsing a scope from a string
#[derive(Debug, Clone)]
pub struct ParseScopeError(String);

impl fmt::Display for ParseScopeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Invalid scope string: {}", self.0)
    }
}

impl std::error::Error for ParseScopeError {}

impl FromStr for ScopeType {
    type Err = ParseScopeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // Accounting scopes
            "accounting.transactions" => Ok(Self::AccountingTransactions(Permission::ReadWrite)),
            "accounting.transactions.read" => {
                Ok(Self::AccountingTransactions(Permission::ReadOnly))
            }
            "accounting.reports.read" => Ok(Self::AccountingReports),
            "accounting.reports.tenninetynine.read" => Ok(Self::AccountingReportsTenninetynine),
            "accounting.budgets.read" => Ok(Self::AccountingBudgets),
            "accounting.journals.read" => Ok(Self::AccountingJournals),
            "accounting.settings" => Ok(Self::AccountingSettings(Permission::ReadWrite)),
            "accounting.settings.read" => Ok(Self::AccountingSettings(Permission::ReadOnly)),
            "accounting.contacts" => Ok(Self::AccountingContacts(Permission::ReadWrite)),
            "accounting.contacts.read" => Ok(Self::AccountingContacts(Permission::ReadOnly)),
            "accounting.attachments" => Ok(Self::AccountingAttachments(Permission::ReadWrite)),
            "accounting.attachments.read" => Ok(Self::AccountingAttachments(Permission::ReadOnly)),

            // Assets scopes
            "assets" => Ok(Self::Assets(Permission::ReadWrite)),
            "assets.read" => Ok(Self::Assets(Permission::ReadOnly)),

            // Files scopes
            "files" => Ok(Self::Files(Permission::ReadWrite)),
            "files.read" => Ok(Self::Files(Permission::ReadOnly)),

            // Payroll scopes
            "payroll.employees" => Ok(Self::PayrollEmployees(Permission::ReadWrite)),
            "payroll.employees.read" => Ok(Self::PayrollEmployees(Permission::ReadOnly)),
            "payroll.payruns" => Ok(Self::PayrollPayruns(Permission::ReadWrite)),
            "payroll.payruns.read" => Ok(Self::PayrollPayruns(Permission::ReadOnly)),
            "payroll.payslip" => Ok(Self::PayrollPayslip(Permission::ReadWrite)),
            "payroll.payslip.read" => Ok(Self::PayrollPayslip(Permission::ReadOnly)),
            "payroll.settings" => Ok(Self::PayrollSettings(Permission::ReadWrite)),
            "payroll.settings.read" => Ok(Self::PayrollSettings(Permission::ReadOnly)),
            "payroll.timesheets" => Ok(Self::PayrollTimesheets(Permission::ReadWrite)),
            "payroll.timesheets.read" => Ok(Self::PayrollTimesheets(Permission::ReadOnly)),

            // Projects scopes
            "projects" => Ok(Self::Projects(Permission::ReadWrite)),
            "projects.read" => Ok(Self::Projects(Permission::ReadOnly)),

            "offline_access" => Ok(Self::OfflineAccess),

            _ => Err(ParseScopeError(s.to_string())),
        }
    }
}

/// A collection of `OAuth2` scopes to request access to Xero APIs
#[derive(Debug, Clone, Default)]
pub struct Scope {
    scopes: Vec<OAuth2Scope>,
}

impl Scope {
    /// Creates a new empty scope collection
    #[must_use]
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    /// Creates a scope collection from a vector of scope types
    #[must_use]
    pub fn from_types(scope_types: Vec<ScopeType>) -> Self {
        let scopes = scope_types
            .into_iter()
            .map(|st| OAuth2Scope::new(st.to_string()))
            .collect();
        Self { scopes }
    }

    /// Creates a scope from a single scope type
    #[must_use]
    pub fn from_type(scope_type: ScopeType) -> Self {
        Self {
            scopes: vec![OAuth2Scope::new(scope_type.to_string())],
        }
    }

    /// Creates a scope from a raw string
    #[must_use]
    pub fn from_string(scope: impl Into<String>) -> Self {
        Self {
            scopes: vec![OAuth2Scope::new(scope.into())],
        }
    }

    /// Add a scope to this collection
    #[must_use]
    pub fn with(mut self, scope_type: ScopeType) -> Self {
        self.scopes.push(OAuth2Scope::new(scope_type.to_string()));
        self
    }

    /// Add multiple scopes to this collection
    #[must_use]
    pub fn with_all(mut self, scope_types: impl IntoIterator<Item = ScopeType>) -> Self {
        for scope_type in scope_types {
            self.scopes.push(OAuth2Scope::new(scope_type.to_string()));
        }
        self
    }

    /// Combine with another scope collection
    #[must_use]
    pub fn combine(mut self, other: Self) -> Self {
        self.scopes.extend(other.scopes);
        self
    }

    /// Converts the scopes into a Vec of `OAuth2` scopes
    #[must_use]
    pub fn into_oauth2_scopes(self) -> Vec<OAuth2Scope> {
        self.scopes
    }

    /// Get a reference to the contained `OAuth2` scopes
    #[must_use]
    pub fn as_oauth2_scopes(&self) -> &[OAuth2Scope] {
        &self.scopes
    }

    /// Convert this scope collection into a single `OAuth2Scope`
    /// for use with the oauth2 crate
    #[must_use]
    pub fn into_oauth2(self) -> OAuth2Scope {
        OAuth2Scope::new(self.to_string())
    }

    // Accounting scopes

    /// Create a scope for full access to transactions
    #[must_use]
    pub fn accounting_transactions() -> Self {
        Self::from_type(ScopeType::AccountingTransactions(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to transactions
    #[must_use]
    pub fn accounting_transactions_read() -> Self {
        Self::from_type(ScopeType::AccountingTransactions(Permission::ReadOnly))
    }

    /// Create a scope for read-only access to reports
    #[must_use]
    pub fn accounting_reports_read() -> Self {
        Self::from_type(ScopeType::AccountingReports)
    }

    /// Create a scope for read-only access to tenninetynine reports
    #[must_use]
    pub fn accounting_reports_tenninetynine_read() -> Self {
        Self::from_type(ScopeType::AccountingReportsTenninetynine)
    }

    /// Create a scope for read-only access to budgets
    #[must_use]
    pub fn accounting_budgets_read() -> Self {
        Self::from_type(ScopeType::AccountingBudgets)
    }

    /// Create a scope for read-only access to journals
    #[must_use]
    pub fn accounting_journals_read() -> Self {
        Self::from_type(ScopeType::AccountingJournals)
    }

    /// Create a scope for full access to settings
    #[must_use]
    pub fn accounting_settings() -> Self {
        Self::from_type(ScopeType::AccountingSettings(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to settings
    #[must_use]
    pub fn accounting_settings_read() -> Self {
        Self::from_type(ScopeType::AccountingSettings(Permission::ReadOnly))
    }

    /// Create a scope for full access to contacts
    #[must_use]
    pub fn accounting_contacts() -> Self {
        Self::from_type(ScopeType::AccountingContacts(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to contacts
    #[must_use]
    pub fn accounting_contacts_read() -> Self {
        Self::from_type(ScopeType::AccountingContacts(Permission::ReadOnly))
    }

    /// Create a scope for full access to attachments
    #[must_use]
    pub fn accounting_attachments() -> Self {
        Self::from_type(ScopeType::AccountingAttachments(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to attachments
    #[must_use]
    pub fn accounting_attachments_read() -> Self {
        Self::from_type(ScopeType::AccountingAttachments(Permission::ReadOnly))
    }

    // Assets scopes

    /// Create a scope for full access to assets
    #[must_use]
    pub fn assets() -> Self {
        Self::from_type(ScopeType::Assets(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to assets
    #[must_use]
    pub fn assets_read() -> Self {
        Self::from_type(ScopeType::Assets(Permission::ReadOnly))
    }

    // Files scopes

    /// Create a scope for full access to files
    #[must_use]
    pub fn files() -> Self {
        Self::from_type(ScopeType::Files(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to files
    #[must_use]
    pub fn files_read() -> Self {
        Self::from_type(ScopeType::Files(Permission::ReadOnly))
    }

    // Payroll scopes

    /// Create a scope for full access to employees
    #[must_use]
    pub fn payroll_employees() -> Self {
        Self::from_type(ScopeType::PayrollEmployees(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to employees
    #[must_use]
    pub fn payroll_employees_read() -> Self {
        Self::from_type(ScopeType::PayrollEmployees(Permission::ReadOnly))
    }

    /// Create a scope for full access to payruns
    #[must_use]
    pub fn payroll_payruns() -> Self {
        Self::from_type(ScopeType::PayrollPayruns(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to payruns
    #[must_use]
    pub fn payroll_payruns_read() -> Self {
        Self::from_type(ScopeType::PayrollPayruns(Permission::ReadOnly))
    }

    /// Create a scope for full access to payslips
    #[must_use]
    pub fn payroll_payslip() -> Self {
        Self::from_type(ScopeType::PayrollPayslip(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to payslips
    #[must_use]
    pub fn payroll_payslip_read() -> Self {
        Self::from_type(ScopeType::PayrollPayslip(Permission::ReadOnly))
    }

    /// Create a scope for full access to payroll settings
    #[must_use]
    pub fn payroll_settings() -> Self {
        Self::from_type(ScopeType::PayrollSettings(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to payroll settings
    #[must_use]
    pub fn payroll_settings_read() -> Self {
        Self::from_type(ScopeType::PayrollSettings(Permission::ReadOnly))
    }

    /// Create a scope for full access to timesheets
    #[must_use]
    pub fn payroll_timesheets() -> Self {
        Self::from_type(ScopeType::PayrollTimesheets(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to timesheets
    #[must_use]
    pub fn payroll_timesheets_read() -> Self {
        Self::from_type(ScopeType::PayrollTimesheets(Permission::ReadOnly))
    }

    // Projects scopes

    /// Create a scope for full access to projects
    #[must_use]
    pub fn projects() -> Self {
        Self::from_type(ScopeType::Projects(Permission::ReadWrite))
    }

    /// Create a scope for read-only access to projects
    #[must_use]
    pub fn projects_read() -> Self {
        Self::from_type(ScopeType::Projects(Permission::ReadOnly))
    }

    /// Shorthand for common accounting scopes (read-only)
    #[must_use]
    pub fn common_accounting_read() -> Self {
        crate::scopes![
            ScopeType::AccountingTransactions(Permission::ReadOnly),
            ScopeType::AccountingReports,
            ScopeType::AccountingContacts(Permission::ReadOnly)
        ]
    }

    /// Shorthand for all accounting scopes (read-only)
    #[must_use]
    pub fn all_accounting_read() -> Self {
        crate::scopes![
            ScopeType::AccountingTransactions(Permission::ReadOnly),
            ScopeType::AccountingReports,
            ScopeType::AccountingReportsTenninetynine,
            ScopeType::AccountingBudgets,
            ScopeType::AccountingJournals,
            ScopeType::AccountingSettings(Permission::ReadOnly),
            ScopeType::AccountingContacts(Permission::ReadOnly),
            ScopeType::AccountingAttachments(Permission::ReadOnly)
        ]
    }

    /// Shorthand for all accounting scopes (with read-write permission)
    #[must_use]
    pub fn all_accounting() -> Self {
        crate::scopes![
            ScopeType::AccountingTransactions(Permission::ReadWrite),
            ScopeType::AccountingReports,
            ScopeType::AccountingReportsTenninetynine,
            ScopeType::AccountingBudgets,
            ScopeType::AccountingJournals,
            ScopeType::AccountingSettings(Permission::ReadWrite),
            ScopeType::AccountingContacts(Permission::ReadWrite),
            ScopeType::AccountingAttachments(Permission::ReadWrite)
        ]
    }

    /// Shorthand for all scopes (read-only)
    #[must_use]
    pub fn all_read() -> Self {
        Self::all_accounting_read()
            .with(ScopeType::Assets(Permission::ReadOnly))
            .with(ScopeType::Files(Permission::ReadOnly))
            .with(ScopeType::PayrollEmployees(Permission::ReadOnly))
            .with(ScopeType::PayrollPayruns(Permission::ReadOnly))
            .with(ScopeType::PayrollPayslip(Permission::ReadOnly))
            .with(ScopeType::PayrollSettings(Permission::ReadOnly))
            .with(ScopeType::PayrollTimesheets(Permission::ReadOnly))
            .with(ScopeType::Projects(Permission::ReadOnly))
    }

    /// Shorthand for all scopes (with read-write permission)
    #[must_use]
    pub fn all() -> Self {
        Self::all_accounting()
            .with(ScopeType::Assets(Permission::ReadWrite))
            .with(ScopeType::Files(Permission::ReadWrite))
            .with(ScopeType::PayrollEmployees(Permission::ReadWrite))
            .with(ScopeType::PayrollPayruns(Permission::ReadWrite))
            .with(ScopeType::PayrollPayslip(Permission::ReadWrite))
            .with(ScopeType::PayrollSettings(Permission::ReadWrite))
            .with(ScopeType::PayrollTimesheets(Permission::ReadWrite))
            .with(ScopeType::Projects(Permission::ReadWrite))
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scopes.is_empty() {
            return write!(f, "");
        }

        let scope_strs: Vec<String> = self.scopes.iter().map(|s| s.to_string()).collect();

        write!(f, "{}", scope_strs.join(" "))
    }
}

impl From<ScopeType> for Scope {
    fn from(scope_type: ScopeType) -> Self {
        Self::from_type(scope_type)
    }
}

impl From<Vec<ScopeType>> for Scope {
    fn from(scope_types: Vec<ScopeType>) -> Self {
        Self::from_types(scope_types)
    }
}

impl From<Scope> for OAuth2Scope {
    fn from(scope: Scope) -> Self {
        scope.into_oauth2()
    }
}

impl From<OAuth2Scope> for Scope {
    fn from(scope: OAuth2Scope) -> Self {
        Self {
            scopes: vec![scope],
        }
    }
}

impl FromIterator<ScopeType> for Scope {
    fn from_iter<I: IntoIterator<Item = ScopeType>>(iter: I) -> Self {
        let scopes = iter
            .into_iter()
            .map(|st| OAuth2Scope::new(st.to_string()))
            .collect();
        Self { scopes }
    }
}

/// Macro to create a complete scope containing multiple scope types
///
/// # Examples
/// ```
/// use xero_rs::{scopes, scope::ScopeType, scope::Permission};
///
/// let scope = scopes![
///     ScopeType::AccountingContacts(Permission::ReadOnly),
///     ScopeType::Files(Permission::ReadWrite)
/// ];
/// ```
#[macro_export]
macro_rules! scopes {
    ($($scope:expr),* $(,)?) => {
        {
            let mut scope = $crate::scope::Scope::new();
            $(
                scope = scope.with($scope);
            )*
            scope
        }
    };
}
