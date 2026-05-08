// Phase: 2
// PromptProfile domain entity

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptProfile {
    pub id: String,
    pub name: String,
    pub instruction_body: String,
    pub is_factory_default: bool,
    pub created_at: String,
    pub updated_at: String,
}
