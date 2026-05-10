// Phase: 9
// Generation commands — job submission, cancellation.
// Uses PromptComposer for real prompt assembly from profiles, tags, and user text.

use tauri::{AppHandle, State};

use crate::contracts::commands::{
    CancelGenerationRequest, StartGenerationResponse, StartInitialRequest, StartRefinementRequest,
};
use crate::contracts::errors::IpcError;
use crate::contracts::shared::RequestKind;
use crate::services::inference::job_coordinator::JobCoordinator;
use crate::services::inference::prompt_composer::{PromptComposer, ResolvedTag};
use crate::services::inference::worker_supervisor::WorkerSupervisor;
use crate::services::persistence::metadata_store::MetadataStore;

fn ensure_requested_model_loaded(
    requested_model_id: &str,
    loaded_model_id: Option<&str>,
) -> Result<(), IpcError> {
    match loaded_model_id {
        None => Err(IpcError {
            code: "MODEL_NOT_READY".to_string(),
            message: "No model is loaded. Select and warm a model before generating.".to_string(),
            detail: None,
            subsystem: "inference".to_string(),
        }),
        Some(loaded) if loaded != requested_model_id => Err(IpcError {
            code: "MODEL_MISMATCH".to_string(),
            message: "Requested model is not loaded. Load the selected model before generating."
                .to_string(),
            detail: Some(format!(
                "Requested model: {}; loaded model: {}",
                requested_model_id, loaded
            )),
            subsystem: "inference".to_string(),
        }),
        Some(_) => Ok(()),
    }
}

/// Resolve active tag IDs into ResolvedTag structs by looking up both
/// built-in and custom tags in the metadata store.
fn resolve_tags(
    metadata_store: &MetadataStore,
    active_tag_ids: &[String],
) -> Result<Vec<ResolvedTag>, IpcError> {
    if active_tag_ids.is_empty() {
        return Ok(Vec::new());
    }

    let (built_in, custom) = metadata_store.list_tags()?;

    let resolved: Vec<ResolvedTag> = active_tag_ids
        .iter()
        .filter_map(|id| {
            if let Some(t) = built_in.iter().find(|t| t.id == *id) {
                Some(ResolvedTag {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    category: t.category.clone(),
                    instruction_body: t.instruction_body.clone(),
                    balancing_group: t.balancing_group.clone(),
                })
            } else if let Some(t) = custom.iter().find(|t| t.id == *id) {
                // Custom tags infer balancing group from category name (lowercase).
                // Whether they actually participate in balancing depends on name matching.
                let group = format!("{:?}", t.category).to_lowercase();
                Some(ResolvedTag {
                    id: t.id.clone(),
                    name: t.name.clone(),
                    category: t.category.clone(),
                    instruction_body: t.instruction_body.clone(),
                    balancing_group: Some(group),
                })
            } else {
                // Unknown tag ID — silently skip (tag may have been deleted)
                log::warn!("Active tag ID not found, skipping: {}", id);
                None
            }
        })
        .collect();

    Ok(resolved)
}

#[tauri::command]
pub async fn generation_start_initial(
    request: StartInitialRequest,
    supervisor: State<'_, WorkerSupervisor>,
    coordinator: State<'_, JobCoordinator>,
    metadata_store: State<'_, MetadataStore>,
    app: AppHandle,
) -> Result<StartGenerationResponse, IpcError> {
    if request.source_text.is_empty() {
        return Err(IpcError {
            code: "EMPTY_INPUT".to_string(),
            message: "Source text must not be empty".to_string(),
            detail: None,
            subsystem: "inference".to_string(),
        });
    }

    let loaded_model_id = supervisor.get_loaded_model_id().await;
    ensure_requested_model_loaded(&request.model_id, loaded_model_id.as_deref())?;

    // Look up profile
    let profile = metadata_store.get_profile_by_id(&request.profile_id)?;

    // Resolve tags
    let tags = resolve_tags(&metadata_store, &request.active_tag_ids)?;

    // Log structured metadata only (P2: never log prompt text)
    log::debug!(
        "Composing initial rewrite prompt: profile={}, tag_count={}, source_len={}",
        profile.id,
        tags.len(),
        request.source_text.len()
    );

    // Compose real prompt
    let prompt_package = PromptComposer::compose_initial_rewrite(
        &profile.instruction_body,
        &tags,
        &request.source_text,
    );

    let job_id = coordinator
        .submit_job(
            &supervisor,
            &app,
            request.tab_id,
            RequestKind::InitialRewrite,
            request.input_version_token,
            None,
            prompt_package,
        )
        .await?;

    Ok(StartGenerationResponse { job_id })
}

#[tauri::command]
pub async fn generation_start_refinement(
    request: StartRefinementRequest,
    supervisor: State<'_, WorkerSupervisor>,
    coordinator: State<'_, JobCoordinator>,
    metadata_store: State<'_, MetadataStore>,
    app: AppHandle,
) -> Result<StartGenerationResponse, IpcError> {
    if request.refinement_instruction.is_empty() {
        return Err(IpcError {
            code: "EMPTY_INSTRUCTION".to_string(),
            message: "Refinement instruction must not be empty".to_string(),
            detail: None,
            subsystem: "inference".to_string(),
        });
    }

    let loaded_model_id = supervisor.get_loaded_model_id().await;
    ensure_requested_model_loaded(&request.model_id, loaded_model_id.as_deref())?;

    // Look up profile
    let profile = metadata_store.get_profile_by_id(&request.profile_id)?;

    // Resolve tags
    let tags = resolve_tags(&metadata_store, &request.active_tag_ids)?;

    // Log structured metadata only (P2: never log prompt text)
    log::debug!(
        "Composing refinement prompt: profile={}, tag_count={}, accepted_len={}, instruction_len={}",
        profile.id,
        tags.len(),
        request.accepted_output.len(),
        request.refinement_instruction.len()
    );

    // Compose real prompt
    let prompt_package = PromptComposer::compose_refinement(
        &profile.instruction_body,
        &tags,
        &request.accepted_output,
        &request.refinement_instruction,
    );

    let job_id = coordinator
        .submit_job(
            &supervisor,
            &app,
            request.tab_id,
            RequestKind::Refinement,
            request.input_version_token,
            Some(request.accepted_output_version),
            prompt_package,
        )
        .await?;

    Ok(StartGenerationResponse { job_id })
}

#[tauri::command]
pub async fn generation_cancel(
    request: CancelGenerationRequest,
    supervisor: State<'_, WorkerSupervisor>,
    coordinator: State<'_, JobCoordinator>,
    app: AppHandle,
) -> Result<(), IpcError> {
    coordinator
        .cancel_job(&supervisor, &app, &request.job_id)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_model_check_accepts_matching_loaded_model() {
        assert!(ensure_requested_model_loaded("model-a", Some("model-a")).is_ok());
    }

    #[test]
    fn requested_model_check_rejects_missing_loaded_model() {
        let err = ensure_requested_model_loaded("model-a", None).unwrap_err();
        assert_eq!(err.code, "MODEL_NOT_READY");
    }

    #[test]
    fn requested_model_check_rejects_stale_loaded_model() {
        let err = ensure_requested_model_loaded("model-a", Some("model-b")).unwrap_err();
        assert_eq!(err.code, "MODEL_MISMATCH");
        assert!(err
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("model-b")));
    }
}
