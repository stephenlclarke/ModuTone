// Phase: 9
// Worker protocol message types
//
// These mirror the backend ↔ worker protocol defined in ipc_contracts.md §4.
// The worker crate does NOT depend on src-tauri — these types are defined
// independently to maintain the binary boundary.

use serde::{Deserialize, Serialize};

// --- Backend → Worker (Inbound) ---

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboundMessage {
    LoadModel {
        #[serde(rename = "modelId")]
        model_id: String,
        #[serde(rename = "modelPath")]
        model_path: String,
    },
    ExecuteJob {
        #[serde(rename = "jobId")]
        job_id: String,
        #[serde(rename = "promptPackage")]
        prompt_package: PromptPackage,
    },
    CancelJob {
        #[serde(rename = "jobId")]
        job_id: String,
    },
    Shutdown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPackage {
    pub system_prompt: String,
    pub user_message: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

// --- Worker → Backend (Outbound) ---

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundMessage {
    Ready,
    ModelLoaded {
        #[serde(rename = "modelId")]
        model_id: String,
        #[serde(rename = "loadTimeMs")]
        load_time_ms: u64,
    },
    ModelLoadFailed {
        #[serde(rename = "modelId")]
        model_id: String,
        error: String,
    },
    JobAck {
        #[serde(rename = "jobId")]
        job_id: String,
    },
    JobProgress {
        #[serde(rename = "jobId")]
        job_id: String,
        #[serde(rename = "partialText")]
        partial_text: String,
        #[serde(rename = "tokenCount")]
        token_count: u32,
    },
    JobCompleted {
        #[serde(rename = "jobId")]
        job_id: String,
        #[serde(rename = "outputText")]
        output_text: String,
        #[serde(rename = "totalTokens")]
        total_tokens: u32,
        #[serde(rename = "durationMs")]
        duration_ms: u64,
    },
    JobFailed {
        #[serde(rename = "jobId")]
        job_id: String,
        error: String,
    },
    JobCanceled {
        #[serde(rename = "jobId")]
        job_id: String,
    },
}
