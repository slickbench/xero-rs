use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum TimesheetStatus {
    #[serde(rename = "DRAFT")]
    Draft,
    #[serde(rename = "APPROVED")]
    Approved,
    #[serde(rename = "PROCESSED")]
    Processed,
}
