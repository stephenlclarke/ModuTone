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
        let api_probe_succeeded = if privacy_blackout_probe_allowed_on_current_platform() {
            use tauri::Manager;
            app.get_webview_window("main")
                .map(|win| win.set_content_protected(false).is_ok())
                .unwrap_or(false)
        } else {
            false
        };

        PlatformCapabilities {
            privacy_blackout_supported: privacy_blackout_supported_from_probe(api_probe_succeeded),
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

fn privacy_blackout_probe_allowed_on_current_platform() -> bool {
    cfg!(any(target_os = "macos", target_os = "windows"))
}

fn privacy_blackout_supported_from_probe(api_probe_succeeded: bool) -> bool {
    privacy_blackout_probe_allowed_on_current_platform() && api_probe_succeeded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_capability_reports_false() {
        assert!(!PlatformCapabilities::unsupported().privacy_blackout_supported);
    }

    #[test]
    fn privacy_blackout_probe_is_gated_by_platform_support() {
        let expected = cfg!(any(target_os = "macos", target_os = "windows"));
        assert_eq!(
            privacy_blackout_probe_allowed_on_current_platform(),
            expected
        );
        assert_eq!(privacy_blackout_supported_from_probe(true), expected);
        assert!(!privacy_blackout_supported_from_probe(false));
    }
}
