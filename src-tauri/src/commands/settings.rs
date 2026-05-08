// Phase: 2

use tauri::State;

use crate::contracts::commands::{
    ModelAliasClearRequest, ModelAliasSetRequest, SettingsGetResponse, SettingsUpdateRequest,
    SettingsUpdateResponse,
};
use crate::contracts::errors::IpcError;
use crate::services::persistence::metadata_store::MetadataStore;

#[tauri::command]
pub async fn settings_get(
    store: State<'_, MetadataStore>,
) -> Result<SettingsGetResponse, IpcError> {
    store.get_settings()
}

#[tauri::command]
pub async fn settings_update(
    request: SettingsUpdateRequest,
    store: State<'_, MetadataStore>,
) -> Result<SettingsUpdateResponse, IpcError> {
    store.update_settings(&request)?;
    Ok(SettingsUpdateResponse { updated: true })
}

#[tauri::command]
pub async fn model_alias_set(
    request: ModelAliasSetRequest,
    store: State<'_, MetadataStore>,
) -> Result<SettingsUpdateResponse, IpcError> {
    store.set_model_alias(&request.model_id, &request.alias)?;
    Ok(SettingsUpdateResponse { updated: true })
}

#[tauri::command]
pub async fn model_alias_clear(
    request: ModelAliasClearRequest,
    store: State<'_, MetadataStore>,
) -> Result<SettingsUpdateResponse, IpcError> {
    store.clear_model_alias(&request.model_id)?;
    Ok(SettingsUpdateResponse { updated: true })
}
