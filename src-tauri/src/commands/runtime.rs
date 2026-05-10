// Phase: 9

use tauri::{AppHandle, State};

use crate::contracts::commands::{RuntimeStatusResponse, WarmModelRequest};
use crate::contracts::errors::IpcError;
use crate::services::inference::model_catalog::{ModelBackend, ModelRegistry};
use crate::services::inference::worker_protocol::WorkerInbound;
use crate::services::inference::worker_supervisor::{WorkerProcessState, WorkerSupervisor};
use crate::services::persistence::metadata_store::MetadataStore;
use crate::services::platform::window_privacy::PlatformCapabilities;

fn runtime_status_response(
    state: WorkerProcessState,
    loaded_model_id: Option<String>,
    metadata_store_writable: bool,
    capabilities: &PlatformCapabilities,
) -> RuntimeStatusResponse {
    RuntimeStatusResponse {
        app_state: state.to_app_state(),
        worker_state: state.to_worker_state(),
        loaded_model_id,
        metadata_store_writable,
        privacy_blackout_supported: capabilities.privacy_blackout_supported,
        tray_supported: false,
        launch_at_login_supported: false,
    }
}

#[tauri::command]
pub async fn runtime_get_status(
    supervisor: State<'_, WorkerSupervisor>,
    capabilities: State<'_, PlatformCapabilities>,
    metadata_store: State<'_, MetadataStore>,
) -> Result<RuntimeStatusResponse, IpcError> {
    let state = supervisor.get_state().await;
    let loaded_model = supervisor.get_loaded_model_id().await;

    Ok(runtime_status_response(
        state,
        loaded_model,
        !metadata_store.is_read_only(),
        &capabilities,
    ))
}

#[tauri::command]
pub async fn runtime_warm_model(
    request: WarmModelRequest,
    supervisor: State<'_, WorkerSupervisor>,
    registry: State<'_, ModelRegistry>,
    app: AppHandle,
) -> Result<(), IpcError> {
    // Check if model is already loaded
    if let Some(ref loaded) = supervisor.get_loaded_model_id().await {
        if *loaded == request.model_id {
            return Err(IpcError {
                code: "MODEL_ALREADY_LOADED".to_string(),
                message: "Model is already loaded".to_string(),
                detail: None,
                subsystem: "inference".to_string(),
            });
        }
    }

    // Look up model in registry and verify it's installed
    let model = registry
        .find_by_id(&request.model_id)
        .ok_or_else(|| IpcError {
            code: "MODEL_NOT_FOUND".to_string(),
            message: format!("Model '{}' not found in catalog", request.model_id),
            detail: None,
            subsystem: "inference".to_string(),
        })?;

    if !model.is_installed {
        return Err(IpcError {
            code: "MODEL_NOT_INSTALLED".to_string(),
            message: format!(
                "Model '{}' is in the catalog but the model files are not present or supported",
                request.model_id
            ),
            detail: Some(format!("Expected at: {}", model.model_path.display())),
            subsystem: "inference".to_string(),
        });
    }

    // Transition to warming (only valid from idle)
    supervisor.set_warming().await.map_err(|e| IpcError {
        code: "WORKER_UNAVAILABLE".to_string(),
        message: "Worker must be idle to warm a model".to_string(),
        detail: Some(e),
        subsystem: "inference".to_string(),
    })?;

    // Send load_model to worker with real model path and backend hint.
    let msg = WorkerInbound::LoadModel {
        model_id: request.model_id.clone(),
        backend: match model.backend {
            ModelBackend::Gguf => crate::services::inference::worker_protocol::ModelBackend::Gguf,
            ModelBackend::Mlx => crate::services::inference::worker_protocol::ModelBackend::Mlx,
        },
        model_path: model.model_path.to_string_lossy().to_string(),
    };

    if let Err(e) = supervisor.send_to_worker(&msg).await {
        // Revert state on send failure
        supervisor.transition_idle_if_busy().await;
        return Err(IpcError {
            code: "WORKER_SEND_FAILED".to_string(),
            message: "Failed to send load_model to worker".to_string(),
            detail: Some(e),
            subsystem: "inference".to_string(),
        });
    }

    supervisor.emit_status_changed(&app, None).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::shared::{AppState, WorkerState};

    #[test]
    fn runtime_status_response_reports_read_only_metadata_store() {
        let response = runtime_status_response(
            WorkerProcessState::Idle,
            Some("test-model".to_string()),
            false,
            &PlatformCapabilities::unsupported(),
        );

        assert_eq!(response.app_state, AppState::Ready);
        assert_eq!(response.worker_state, WorkerState::Idle);
        assert_eq!(response.loaded_model_id.as_deref(), Some("test-model"));
        assert!(!response.metadata_store_writable);
        assert!(!response.privacy_blackout_supported);
        assert!(!response.tray_supported);
        assert!(!response.launch_at_login_supported);
    }
}
