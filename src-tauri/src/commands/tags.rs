// Phase: 2

use tauri::State;

use crate::contracts::commands::{
    TagCreateRequest, TagCreateResponse, TagDeleteRequest, TagDeleteResponse, TagUpdateRequest,
    TagUpdateResponse, TagsListResponse,
};
use crate::contracts::errors::IpcError;
use crate::services::persistence::metadata_store::MetadataStore;

#[tauri::command]
pub async fn tags_list(store: State<'_, MetadataStore>) -> Result<TagsListResponse, IpcError> {
    let (built_in_tags, custom_tags) = store.list_tags()?;
    Ok(TagsListResponse {
        built_in_tags,
        custom_tags,
    })
}

#[tauri::command]
pub async fn tags_create(
    request: TagCreateRequest,
    store: State<'_, MetadataStore>,
) -> Result<TagCreateResponse, IpcError> {
    store.create_tag(&request)
}

#[tauri::command]
pub async fn tags_update(
    request: TagUpdateRequest,
    store: State<'_, MetadataStore>,
) -> Result<TagUpdateResponse, IpcError> {
    store.update_tag(&request)?;
    Ok(TagUpdateResponse { updated: true })
}

#[tauri::command]
pub async fn tags_delete(
    request: TagDeleteRequest,
    store: State<'_, MetadataStore>,
) -> Result<TagDeleteResponse, IpcError> {
    store.delete_tag(&request.id)?;
    Ok(TagDeleteResponse { deleted: true })
}
