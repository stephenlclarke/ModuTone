// Phase: 2
// Tag domain entities

use serde::{Deserialize, Serialize};

use crate::contracts::shared::TagCategory;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltInTag {
    pub id: String,
    pub name: String,
    pub category: TagCategory,
    pub instruction_body: String,
    pub is_built_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balancing_group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomTag {
    pub id: String,
    pub name: String,
    pub category: TagCategory,
    pub instruction_body: String,
    pub is_built_in: bool,
    pub created_at: String,
    pub updated_at: String,
}
