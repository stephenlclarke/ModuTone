// Phase: 9

use tauri::{AppHandle, State};

use crate::contracts::commands::{RuntimeStatusResponse, WarmModelRequest};
use crate::contracts::errors::IpcError;
use crate::services::inference::model_catalog::ModelRegistry;
use crate::services::inference::worker_protocol::WorkerInbound;
use crate::services::inference::worker_supervisor::WorkerSupervisor;
use crate::services::platform::window_privacy::PlatformCapabilities;

#[tauri::command]
pub async fn runtime_get_status(
    supervisor: State<'_, WorkerSupervisor>,
    capabilities: State<'_, PlatformCapabilities>,
) -> Result<RuntimeStatusResponse, IpcError> {
    let state = supervisor.get_state().await;
    let loaded_model = supervisor.get_loaded_model_id().await;

    Ok(RuntimeStatusResponse {
        app_state: state.to_app_state(),
        worker_state: state.to_worker_state(),
        loaded_model_id: loaded_model,
        metadata_store_writable: true,
        privacy_blackout_supported: capabilities.privacy_blackout_supported,
        tray_supported: false,
        launch_at_login_supported: false,
    })
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
                "Model '{}' is in the catalog but the GGUF file is not present on disk",
                request.model_id
            ),
            detail: Some(format!("Expected at: {}", model.gguf_path.display())),
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

    // Send load_model to worker with real GGUF path
    let msg = WorkerInbound::LoadModel {
        model_id: request.model_id.clone(),
        model_path: model.gguf_path.to_string_lossy().to_string(),
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
