// Phase: 4
// Model catalog domain entity

use serde::{Deserialize, Serialize};

use crate::contracts::shared::ModelSuitability;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub ram_class_label: String,
    pub min_ram_bytes: u64,
    pub is_installed: bool,
    pub suitability: ModelSuitability,
}
