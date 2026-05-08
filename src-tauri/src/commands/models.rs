// Phase: 9

use sysinfo::System;
use tauri::State;

use crate::contracts::commands::{ModelEntry, ModelsListResponse};
use crate::contracts::errors::IpcError;
use crate::contracts::shared::ModelSuitability;
use crate::services::inference::model_catalog::ModelRegistry;

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
    registry: State<'_, ModelRegistry>,
) -> Result<ModelsListResponse, IpcError> {
    let system_ram_bytes = get_system_ram_bytes();

    let models: Vec<ModelEntry> = registry
        .models()
        .iter()
        .map(|m| ModelEntry {
            id: m.id.clone(),
            display_name: m.display_name.clone(),
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
        })
        .collect();

    Ok(ModelsListResponse {
        models,
        system_ram_bytes,
        system_vram_bytes: None,
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
