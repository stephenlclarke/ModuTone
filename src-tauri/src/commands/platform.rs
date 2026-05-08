// Phase: 8

use tauri::{AppHandle, Manager, State};

use crate::contracts::commands::{PlatformFeatureResponse, SetBooleanRequest};
use crate::contracts::errors::IpcError;
use crate::services::platform::window_privacy::PlatformCapabilities;

#[tauri::command]
pub async fn app_set_launch_at_login(
    request: SetBooleanRequest,
) -> Result<PlatformFeatureResponse, IpcError> {
    let _ = request;
    Ok(PlatformFeatureResponse {
        applied: false,
        supported: false,
    })
}

#[tauri::command]
pub async fn app_set_tray_enabled(
    request: SetBooleanRequest,
) -> Result<PlatformFeatureResponse, IpcError> {
    let _ = request;
    Ok(PlatformFeatureResponse {
        applied: false,
        supported: false,
    })
}

#[tauri::command]
pub async fn app_set_privacy_blackout(
    request: SetBooleanRequest,
    app: AppHandle,
    capabilities: State<'_, PlatformCapabilities>,
) -> Result<PlatformFeatureResponse, IpcError> {
    if !capabilities.privacy_blackout_supported {
        return Ok(PlatformFeatureResponse {
            applied: false,
            supported: false,
        });
    }

    let result = app
        .get_webview_window("main")
        .map(|win| win.set_content_protected(request.enabled).is_ok())
        .unwrap_or(false);

    Ok(PlatformFeatureResponse {
        applied: result,
        supported: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tray_stub_returns_unsupported() {
        let req = SetBooleanRequest {
            contract_version: 1,
            enabled: true,
        };
        let resp = app_set_tray_enabled(req).await.unwrap();
        assert!(!resp.applied);
        assert!(!resp.supported);
    }

    #[tokio::test]
    async fn launch_at_login_stub_returns_unsupported() {
        let req = SetBooleanRequest {
            contract_version: 1,
            enabled: true,
        };
        let resp = app_set_launch_at_login(req).await.unwrap();
        assert!(!resp.applied);
        assert!(!resp.supported);
    }
}
