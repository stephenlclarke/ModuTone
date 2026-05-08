// Phase: 4
// Job domain entity and state machine

use serde::{Deserialize, Serialize};

use crate::contracts::shared::RequestKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Acknowledged,
    Running,
    Completed,
    Failed,
    Canceled,
    StaleDiscarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub tab_id: String,
    pub request_kind: RequestKind,
    pub input_version_token: String,
    pub status: JobStatus,
}
