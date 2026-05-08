// Phase: 2

use tauri::State;

use crate::contracts::commands::{
    ProfileCreateRequest, ProfileCreateResponse, ProfileDeleteRequest, ProfileDeleteResponse,
    ProfileResetRequest, ProfileResetResponse, ProfileUpdateRequest, ProfileUpdateResponse,
    ProfilesListResponse,
};
use crate::contracts::errors::IpcError;
use crate::services::persistence::metadata_store::MetadataStore;

#[tauri::command]
pub async fn profiles_list(
    store: State<'_, MetadataStore>,
) -> Result<ProfilesListResponse, IpcError> {
    let profiles = store.list_profiles()?;
    Ok(ProfilesListResponse { profiles })
}

#[tauri::command]
pub async fn profiles_create(
    request: ProfileCreateRequest,
    store: State<'_, MetadataStore>,
) -> Result<ProfileCreateResponse, IpcError> {
    store.create_profile(&request)
}

#[tauri::command]
pub async fn profiles_update(
    request: ProfileUpdateRequest,
    store: State<'_, MetadataStore>,
) -> Result<ProfileUpdateResponse, IpcError> {
    store.update_profile(&request)?;
    Ok(ProfileUpdateResponse { updated: true })
}

#[tauri::command]
pub async fn profiles_delete(
    request: ProfileDeleteRequest,
    store: State<'_, MetadataStore>,
) -> Result<ProfileDeleteResponse, IpcError> {
    store.delete_profile(&request.id)?;
    Ok(ProfileDeleteResponse { deleted: true })
}

#[tauri::command]
pub async fn profiles_reset_to_default(
    request: ProfileResetRequest,
    store: State<'_, MetadataStore>,
) -> Result<ProfileResetResponse, IpcError> {
    store.reset_profile_to_default(&request.id)?;
    Ok(ProfileResetResponse { reset: true })
}
