use serde::Deserialize;
use uuid::Uuid;

/// Represents a leave type in Xero Payroll AU
///
/// Leave types define the categories of leave available to employees,
/// such as Annual Leave, Personal/Carer's Leave, etc.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct LeaveType {
    /// Unique identifier for the leave type
    #[serde(rename = "LeaveTypeID")]
    pub leave_type_id: Uuid,

    /// Name of the leave type (e.g., "Annual Leave", "Personal/Carer's Leave")
    pub name: String,

    /// Unit type for the leave ("Hours" or "Days")
    #[serde(default)]
    pub type_of_units: Option<String>,

    /// Whether this leave type is paid
    #[serde(default)]
    pub is_paid_leave: Option<bool>,

    /// Whether this leave appears on payslips
    #[serde(default)]
    pub show_on_payslip: Option<bool>,

    /// The leave loading percentage (e.g., 17.5 for annual leave loading)
    #[serde(default)]
    pub leave_loading_rate: Option<f64>,

    /// Normal entitlement in units per year
    #[serde(default)]
    pub normal_entitlement: Option<f64>,

    /// Whether leave balance shows on payslip
    #[serde(default)]
    pub show_balance_on_payslip: Option<bool>,

    /// Account code for leave liability
    pub leave_category_code: Option<String>,

    /// Whether the leave type is active
    #[serde(default)]
    pub current_record: Option<bool>,
}
