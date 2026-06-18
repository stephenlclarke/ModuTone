// Phase: 4
// Backend-side worker protocol message types.
//
// These mirror the worker's protocol.rs types but are defined independently
// to maintain the binary boundary (the two crates do not share code).
// Inbound messages (backend → worker) derive Serialize.
// Outbound messages (worker → backend) derive Deserialize.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelBackend {
    Gguf,
    Mlx,
}

// --- Backend → Worker (Inbound) ---

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerInbound {
    LoadModel {
        #[serde(rename = "modelId")]
        model_id: String,
        backend: ModelBackend,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPackage {
    pub system_prompt: String,
    pub user_message: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

// --- Worker → Backend (Outbound) ---

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerOutbound {
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
