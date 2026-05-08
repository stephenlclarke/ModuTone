// Phase: 8

/// Stores the runtime-detected privacy blackout capability.
/// Probed once at app startup by attempting the API call.
#[derive(Debug, Clone)]
pub struct PlatformCapabilities {
    pub privacy_blackout_supported: bool,
}

impl PlatformCapabilities {
    /// Probe privacy blackout support by attempting `set_content_protected(false)`.
    /// Returns capabilities with the detected support status.
    pub fn probe(app: &tauri::AppHandle) -> Self {
        use tauri::Manager;
        let supported = app
            .get_webview_window("main")
            .map(|win| win.set_content_protected(false).is_ok())
            .unwrap_or(false);

        PlatformCapabilities {
            privacy_blackout_supported: supported,
        }
    }

    /// Create capabilities indicating no support (for testing or fallback).
    #[cfg(test)]
    pub fn unsupported() -> Self {
        PlatformCapabilities {
            privacy_blackout_supported: false,
        }
    }
}
