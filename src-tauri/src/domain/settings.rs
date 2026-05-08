// Phase: 2
// Settings domain entity

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::contracts::shared::{MotionPreference, ThemePreference, VisualStyle};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub theme_preference: ThemePreference,
    pub tray_enabled: bool,
    pub launch_at_login: bool,
    pub privacy_blackout_enabled: bool,
    pub selected_model_id: Option<String>,
    pub last_selected_profile_id: Option<String>,
    #[serde(default)]
    pub last_successful_model_id: Option<String>,
    #[serde(default)]
    pub visual_style: VisualStyle,
    #[serde(default)]
    pub motion_preference: MotionPreference,
    #[serde(default)]
    pub model_aliases: HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            theme_preference: ThemePreference::System,
            tray_enabled: false,
            launch_at_login: false,
            privacy_blackout_enabled: false,
            selected_model_id: None,
            last_selected_profile_id: None,
            last_successful_model_id: None,
            visual_style: VisualStyle::default(),
            motion_preference: MotionPreference::default(),
            model_aliases: HashMap::new(),
        }
    }
}
