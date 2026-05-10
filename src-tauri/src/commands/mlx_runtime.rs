// MLX runtime setup commands.

use tauri::{AppHandle, State};

use crate::contracts::commands::{
    MlxRuntimeInstallStartRequest, MlxRuntimeInstallStartResponse, MlxRuntimeStatusResponse,
};
use crate::contracts::errors::IpcError;
use crate::services::inference::mlx_runtime::MlxRuntimeManager;

#[tauri::command]
pub async fn mlx_runtime_status(
    manager: State<'_, MlxRuntimeManager>,
) -> Result<MlxRuntimeStatusResponse, IpcError> {
    let status = manager.status().await;
    Ok(MlxRuntimeStatusResponse {
        supported: status.supported,
        installed: status.installed,
        installing: status.installing,
        install_dir: status.install_dir.to_string_lossy().to_string(),
        python_path: status
            .python_path
            .map(|path| path.to_string_lossy().to_string()),
        unavailable_reason: status.unavailable_reason,
    })
}

#[tauri::command]
pub async fn mlx_runtime_install_start(
    app: AppHandle,
    manager: State<'_, MlxRuntimeManager>,
    request: MlxRuntimeInstallStartRequest,
) -> Result<MlxRuntimeInstallStartResponse, IpcError> {
    if request.contract_version != 1 {
        return Err(IpcError {
            code: "INVALID_CONTRACT_VERSION".to_string(),
            message: "Unsupported MLX runtime install contract version".to_string(),
            detail: Some(request.contract_version.to_string()),
            subsystem: "models".to_string(),
        });
    }

    let result = manager.start_install(app).await.map_err(|e| IpcError {
        code: "MLX_RUNTIME_INSTALL_FAILED_TO_START".to_string(),
        message: "Failed to start MLX runtime setup".to_string(),
        detail: Some(e),
        subsystem: "models".to_string(),
    })?;

    Ok(MlxRuntimeInstallStartResponse {
        started: result.started,
        already_installed: result.already_installed,
        install_dir: result.install_dir.to_string_lossy().to_string(),
        python_path: result
            .python_path
            .map(|path| path.to_string_lossy().to_string()),
    })
}
