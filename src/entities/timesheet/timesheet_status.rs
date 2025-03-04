use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub enum TimesheetStatus {
    Draft,
    Approved,
    Processed,
} 