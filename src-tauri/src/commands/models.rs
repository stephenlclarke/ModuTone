// Phase: 9

use std::sync::{Arc, Mutex};

use sysinfo::System;
use tauri::{AppHandle, State};

use crate::contracts::commands::{
    ensure_contract_version, ModelDownloadCancelRequest, ModelDownloadCancelResponse,
    ModelDownloadStartRequest, ModelDownloadStartResponse, ModelEntry, ModelsListResponse,
};
use crate::contracts::errors::IpcError;
use crate::contracts::shared::ModelSuitability;
use crate::services::inference::model_catalog::{ModelBackend, ModelRegistry};
use crate::services::inference::model_downloader::{download_spec_for_model, ModelDownloadManager};

fn compute_suitability(system_ram_bytes: u64, min_ram_bytes: u64) -> ModelSuitability {
    let threshold_recommended = min_ram_bytes + min_ram_bytes / 2; // 1.5x
    if system_ram_bytes >= threshold_recommended {
        ModelSuitability::Recommended
    } else if system_ram_bytes >= min_ram_bytes {
        ModelSuitability::Caution
    } else {
        ModelSuitability::Unsupported
    }
}

fn get_system_ram_bytes() -> u64 {
    let sys = System::new_with_specifics(
        sysinfo::RefreshKind::nothing().with_memory(sysinfo::MemoryRefreshKind::everything()),
    );
    sys.total_memory()
}

#[tauri::command]
pub async fn models_list(
    registry: State<'_, Arc<Mutex<ModelRegistry>>>,
) -> Result<ModelsListResponse, IpcError> {
    let system_ram_bytes = get_system_ram_bytes();
    let registry = registry.lock().map_err(|e| IpcError {
        code: "MODEL_REGISTRY_LOCK_FAILED".to_string(),
        message: "Failed to access model registry".to_string(),
        detail: Some(e.to_string()),
        subsystem: "models".to_string(),
    })?;

    let models: Vec<ModelEntry> = registry
        .models()
        .iter()
        .map(|m| {
            let download_spec = download_spec_for_model(&m.id);
            let can_download = !m.is_installed
                && download_spec
                    .as_ref()
                    .is_some_and(|spec| spec.can_download());

            ModelEntry {
                id: m.id.clone(),
                display_name: m.display_name.clone(),
                backend: match m.backend {
                    ModelBackend::Gguf => "gguf".to_string(),
                    ModelBackend::Mlx => "mlx".to_string(),
                },
                size_bytes: m.size_bytes,
                ram_class_label: m.ram_class_label.clone(),
                min_ram_bytes: m.min_ram_bytes,
                is_installed: m.is_installed,
                is_cataloged: m.is_cataloged,
                // Uncataloged models always get Recommended — the RAM estimate
                // is display-only guidance, not a compatibility gate.
                suitability: if m.is_cataloged {
                    compute_suitability(system_ram_bytes, m.min_ram_bytes)
                } else {
                    ModelSuitability::Recommended
                },
                quant_label: m.quant_label.clone(),
                can_download,
                download_size_bytes: download_spec.as_ref().map(|spec| spec.size_bytes),
                download_unavailable_reason: download_spec
                    .and_then(|spec| spec.unsupported_reason.map(ToString::to_string)),
            }
        })
        .collect();

    Ok(ModelsListResponse {
        models,
        system_ram_bytes,
        system_vram_bytes: None,
    })
}

#[tauri::command]
pub async fn model_download_start(
    app: AppHandle,
    registry: State<'_, Arc<Mutex<ModelRegistry>>>,
    manager: State<'_, ModelDownloadManager>,
    request: ModelDownloadStartRequest,
) -> Result<ModelDownloadStartResponse, IpcError> {
    ensure_contract_version(request.contract_version, "models")?;
    let registry = registry.inner().clone();
    let result = manager
        .start(app, registry, request.model_id.clone())
        .await
        .map_err(|e| IpcError {
            code: "MODEL_DOWNLOAD_FAILED_TO_START".to_string(),
            message: format!("Failed to start model download '{}'", request.model_id),
            detail: Some(e),
            subsystem: "models".to_string(),
        })?;

    Ok(ModelDownloadStartResponse {
        started: result.started,
        already_installed: result.already_installed,
        total_bytes: result.total_bytes,
    })
}

#[tauri::command]
pub async fn model_download_cancel(
    manager: State<'_, ModelDownloadManager>,
    request: ModelDownloadCancelRequest,
) -> Result<ModelDownloadCancelResponse, IpcError> {
    ensure_contract_version(request.contract_version, "models")?;
    Ok(ModelDownloadCancelResponse {
        canceled: manager.cancel(&request.model_id).await,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suitability_recommended_when_plenty_of_ram() {
        // 24 GB system, 8 GB minimum → 24 >= 12 (1.5x) → recommended
        assert_eq!(
            compute_suitability(24_000_000_000, 8_000_000_000),
            ModelSuitability::Recommended
        );
    }

    #[test]
    fn suitability_caution_when_tight() {
        // 10 GB system, 8 GB minimum → 10 >= 8 but 10 < 12 → caution
        assert_eq!(
            compute_suitability(10_000_000_000, 8_000_000_000),
            ModelSuitability::Caution
        );
    }

    #[test]
    fn suitability_unsupported_when_insufficient() {
        // 6 GB system, 8 GB minimum → 6 < 8 → unsupported
        assert_eq!(
            compute_suitability(6_000_000_000, 8_000_000_000),
            ModelSuitability::Unsupported
        );
    }

    #[test]
    fn suitability_exact_boundary_at_min_is_caution() {
        // Exactly at min_ram_bytes → caution (not unsupported)
        assert_eq!(
            compute_suitability(8_000_000_000, 8_000_000_000),
            ModelSuitability::Caution
        );
    }

    #[test]
    fn suitability_exact_boundary_at_1_5x_is_recommended() {
        // Exactly at 1.5x → recommended
        assert_eq!(
            compute_suitability(12_000_000_000, 8_000_000_000),
            ModelSuitability::Recommended
        );
    }

    #[test]
    fn system_ram_detection_returns_nonzero() {
        let ram = get_system_ram_bytes();
        assert!(ram > 0, "System RAM should be detected as nonzero");
    }
}
