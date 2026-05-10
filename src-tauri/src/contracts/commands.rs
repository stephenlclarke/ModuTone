// Phase: 1
// Command request/response types for all IPC commands

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::errors::IpcError;
use super::shared::{
    AppState, ModelSuitability, MotionPreference, TagCategory, ThemePreference, VisualStyle,
    WorkerState,
};

pub const CONTRACT_VERSION: u32 = 1;

pub fn ensure_contract_version(contract_version: u32, subsystem: &str) -> Result<(), IpcError> {
    if contract_version == CONTRACT_VERSION {
        return Ok(());
    }

    Err(IpcError {
        code: "INVALID_CONTRACT_VERSION".to_string(),
        message: format!(
            "Unsupported IPC contract version {}; expected {}",
            contract_version, CONTRACT_VERSION
        ),
        detail: Some(contract_version.to_string()),
        subsystem: subsystem.to_string(),
    })
}

// --- Settings ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsGetResponse {
    pub schema_version: u32,
    pub theme_preference: ThemePreference,
    pub tray_enabled: bool,
    pub launch_at_login: bool,
    pub privacy_blackout_enabled: bool,
    pub selected_model_id: Option<String>,
    pub last_selected_profile_id: Option<String>,
    pub last_successful_model_id: Option<String>,
    pub visual_style: VisualStyle,
    pub motion_preference: MotionPreference,
    pub model_aliases: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdateRequest {
    pub contract_version: u32,
    pub theme_preference: Option<ThemePreference>,
    pub tray_enabled: Option<bool>,
    pub launch_at_login: Option<bool>,
    pub privacy_blackout_enabled: Option<bool>,
    pub selected_model_id: Option<Option<String>>,
    pub last_selected_profile_id: Option<Option<String>>,
    pub last_successful_model_id: Option<Option<String>>,
    pub visual_style: Option<VisualStyle>,
    pub motion_preference: Option<MotionPreference>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdateResponse {
    pub updated: bool,
}

// --- Model Aliases ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAliasSetRequest {
    pub contract_version: u32,
    pub model_id: String,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelAliasClearRequest {
    pub contract_version: u32,
    pub model_id: String,
}

// --- Profiles ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileEntry {
    pub id: String,
    pub name: String,
    pub instruction_body: String,
    pub is_factory_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesListResponse {
    pub profiles: Vec<ProfileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCreateRequest {
    pub contract_version: u32,
    pub name: String,
    pub instruction_body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCreateResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpdateRequest {
    pub contract_version: u32,
    pub id: String,
    pub name: Option<String>,
    pub instruction_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileUpdateResponse {
    pub updated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDeleteRequest {
    pub contract_version: u32,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDeleteResponse {
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResetRequest {
    pub contract_version: u32,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResetResponse {
    pub reset: bool,
}

// --- Tags ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltInTagEntry {
    pub id: String,
    pub name: String,
    pub category: TagCategory,
    pub instruction_body: String,
    pub is_built_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balancing_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTagEntry {
    pub id: String,
    pub name: String,
    pub category: TagCategory,
    pub instruction_body: String,
    pub is_built_in: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagsListResponse {
    pub built_in_tags: Vec<BuiltInTagEntry>,
    pub custom_tags: Vec<CustomTagEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCreateRequest {
    pub contract_version: u32,
    pub name: String,
    pub category: TagCategory,
    pub instruction_body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCreateResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagUpdateRequest {
    pub contract_version: u32,
    pub id: String,
    pub name: Option<String>,
    pub category: Option<TagCategory>,
    pub instruction_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagUpdateResponse {
    pub updated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDeleteRequest {
    pub contract_version: u32,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDeleteResponse {
    pub deleted: bool,
}

// --- Models ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEntry {
    pub id: String,
    pub display_name: String,
    pub backend: String,
    pub size_bytes: u64,
    pub ram_class_label: String,
    pub min_ram_bytes: u64,
    pub is_installed: bool,
    pub is_cataloged: bool,
    pub suitability: ModelSuitability,
    pub quant_label: Option<String>,
    pub can_download: bool,
    pub download_size_bytes: Option<u64>,
    pub download_unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsListResponse {
    pub models: Vec<ModelEntry>,
    pub system_ram_bytes: u64,
    pub system_vram_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadStartRequest {
    pub contract_version: u32,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadStartResponse {
    pub started: bool,
    pub already_installed: bool,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadCancelRequest {
    pub contract_version: u32,
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadCancelResponse {
    pub canceled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MlxRuntimeStatusResponse {
    pub supported: bool,
    pub installed: bool,
    pub installing: bool,
    pub install_dir: String,
    pub python_path: Option<String>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MlxRuntimeInstallStartRequest {
    pub contract_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MlxRuntimeInstallStartResponse {
    pub started: bool,
    pub already_installed: bool,
    pub install_dir: String,
    pub python_path: Option<String>,
}

// --- Runtime ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatusResponse {
    pub app_state: AppState,
    pub worker_state: WorkerState,
    pub loaded_model_id: Option<String>,
    pub metadata_store_writable: bool,
    pub privacy_blackout_supported: bool,
    pub tray_supported: bool,
    pub launch_at_login_supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarmModelRequest {
    pub contract_version: u32,
    pub model_id: String,
}

// --- Generation ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartInitialRequest {
    pub contract_version: u32,
    pub tab_id: String,
    pub model_id: String,
    pub profile_id: String,
    pub active_tag_ids: Vec<String>,
    pub source_text: String,
    pub input_version_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartRefinementRequest {
    pub contract_version: u32,
    pub tab_id: String,
    pub model_id: String,
    pub profile_id: String,
    pub active_tag_ids: Vec<String>,
    pub accepted_output: String,
    pub accepted_output_version: u32,
    pub refinement_instruction: String,
    pub input_version_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartGenerationResponse {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelGenerationRequest {
    pub contract_version: u32,
    pub job_id: String,
    pub tab_id: String,
}

// --- Platform ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBooleanRequest {
    pub contract_version: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformFeatureResponse {
    pub applied: bool,
    pub supported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_version_validator_accepts_current_version() {
        assert!(ensure_contract_version(CONTRACT_VERSION, "test").is_ok());
    }

    #[test]
    fn contract_version_validator_rejects_unsupported_version() {
        let err = ensure_contract_version(CONTRACT_VERSION + 1, "test").unwrap_err();

        assert_eq!(err.code, "INVALID_CONTRACT_VERSION");
        assert_eq!(err.subsystem, "test");
        assert!(err.message.contains("Unsupported IPC contract version"));
    }
}
