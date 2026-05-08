// Phase: 1
// Event payload types for backend → frontend events

use serde::{Deserialize, Serialize};

use super::errors::IpcError;
use super::shared::{AppState, RequestKind, WorkerState};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusChangedEvent {
    pub contract_version: u32,
    pub app_state: AppState,
    pub worker_state: WorkerState,
    pub loaded_model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_error_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationStartedEvent {
    pub contract_version: u32,
    pub job_id: String,
    pub tab_id: String,
    pub request_kind: RequestKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationProgressEvent {
    pub contract_version: u32,
    pub job_id: String,
    pub tab_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationCompletedEvent {
    pub contract_version: u32,
    pub job_id: String,
    pub tab_id: String,
    pub request_kind: RequestKind,
    pub input_version_token: String,
    pub accepted_output_version: Option<u32>,
    pub output_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationFailedEvent {
    pub contract_version: u32,
    pub job_id: String,
    pub tab_id: String,
    pub request_kind: RequestKind,
    pub error: IpcError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationCanceledEvent {
    pub contract_version: u32,
    pub job_id: String,
    pub tab_id: String,
    pub request_kind: RequestKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerCrashedEvent {
    pub contract_version: u32,
    pub restart_attempt: u32,
    pub will_restart: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacySupportStatusChangedEvent {
    pub contract_version: u32,
    pub privacy_blackout_supported: bool,
    pub platform: String,
}
