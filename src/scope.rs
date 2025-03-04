use oauth2::Scope as OAuth2Scope;

/// Represents a Xero API scope.
#[derive(Debug, Clone)]
pub struct XeroScope(OAuth2Scope);

impl XeroScope {
    /// Creates a new scope from a string.
    #[must_use] pub fn new(scope: String) -> Self {
        Self(OAuth2Scope::new(scope))
    }

    /// Converts the scope into an `OAuth2` scope.
    #[must_use] pub fn into_oauth2(self) -> OAuth2Scope {
        self.0
    }

    // Accounting scopes
    #[must_use] pub fn accounting_transactions() -> Self {
        Self::new("accounting.transactions".to_string())
    }

    #[must_use] pub fn accounting_transactions_read() -> Self {
        Self::new("accounting.transactions.read".to_string())
    }

    #[must_use] pub fn accounting_reports_read() -> Self {
        Self::new("accounting.reports.read".to_string())
    }

    #[must_use] pub fn accounting_reports_tenninetynine_read() -> Self {
        Self::new("accounting.reports.tenninetynine.read".to_string())
    }

    #[must_use] pub fn accounting_budgets_read() -> Self {
        Self::new("accounting.budgets.read".to_string())
    }

    #[must_use] pub fn accounting_journals_read() -> Self {
        Self::new("accounting.journals.read".to_string())
    }

    #[must_use] pub fn accounting_settings() -> Self {
        Self::new("accounting.settings".to_string())
    }

    #[must_use] pub fn accounting_settings_read() -> Self {
        Self::new("accounting.settings.read".to_string())
    }

    #[must_use] pub fn accounting_contacts() -> Self {
        Self::new("accounting.contacts".to_string())
    }

    #[must_use] pub fn accounting_contacts_read() -> Self {
        Self::new("accounting.contacts.read".to_string())
    }

    #[must_use] pub fn accounting_attachments() -> Self {
        Self::new("accounting.attachments".to_string())
    }

    #[must_use] pub fn accounting_attachments_read() -> Self {
        Self::new("accounting.attachments.read".to_string())
    }

    // Assets scopes
    #[must_use] pub fn assets() -> Self {
        Self::new("assets".to_string())
    }

    #[must_use] pub fn assets_read() -> Self {
        Self::new("assets.read".to_string())
    }

    // Files scopes
    #[must_use] pub fn files() -> Self {
        Self::new("files".to_string())
    }

    #[must_use] pub fn files_read() -> Self {
        Self::new("files.read".to_string())
    }

    // Payroll scopes
    #[must_use] pub fn payroll_employees() -> Self {
        Self::new("payroll.employees".to_string())
    }

    #[must_use] pub fn payroll_employees_read() -> Self {
        Self::new("payroll.employees.read".to_string())
    }

    #[must_use] pub fn payroll_payruns() -> Self {
        Self::new("payroll.payruns".to_string())
    }

    #[must_use] pub fn payroll_payruns_read() -> Self {
        Self::new("payroll.payruns.read".to_string())
    }

    #[must_use] pub fn payroll_payslip() -> Self {
        Self::new("payroll.payslip".to_string())
    }

    #[must_use] pub fn payroll_payslip_read() -> Self {
        Self::new("payroll.payslip.read".to_string())
    }

    #[must_use] pub fn payroll_settings() -> Self {
        Self::new("payroll.settings".to_string())
    }

    #[must_use] pub fn payroll_settings_read() -> Self {
        Self::new("payroll.settings.read".to_string())
    }

    #[must_use] pub fn payroll_timesheets() -> Self {
        Self::new("payroll.timesheets".to_string())
    }

    #[must_use] pub fn payroll_timesheets_read() -> Self {
        Self::new("payroll.timesheets.read".to_string())
    }

    // Projects scopes
    #[must_use] pub fn projects() -> Self {
        Self::new("projects".to_string())
    }

    #[must_use] pub fn projects_read() -> Self {
        Self::new("projects.read".to_string())
    }
}

impl From<XeroScope> for OAuth2Scope {
    fn from(scope: XeroScope) -> Self {
        scope.into_oauth2()
    }
}

impl From<OAuth2Scope> for XeroScope {
    fn from(scope: OAuth2Scope) -> Self {
        Self(scope)
    }
} 