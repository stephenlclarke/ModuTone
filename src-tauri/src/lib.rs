// Phase: 9
// Library root for ModuTone backend

pub mod app;
pub mod commands;
pub mod contracts;
pub mod domain;
pub mod infrastructure;
pub mod services;

use commands::{generation, models, platform, profiles, runtime, settings, tags};
use services::diagnostics::init_logger;
use services::inference::job_coordinator::JobCoordinator;
use services::inference::model_catalog::ModelRegistry;
use services::inference::worker_supervisor::{resolve_worker_binary_path, WorkerSupervisor};
use services::persistence::metadata_store::MetadataStore;
use services::platform::window_privacy::PlatformCapabilities;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            use tauri::Manager;

            // Resolve app data directory
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to resolve app data directory: {}", e))?;

            // Initialize file logger before anything else
            if let Err(e) = init_logger(&data_dir) {
                eprintln!("Logger initialization failed (non-fatal): {}", e);
            }
            let store = MetadataStore::init(&data_dir)
                .map_err(|e| format!("Failed to initialize metadata store: {}", e))?;
            app.manage(store);

            // Initialize model registry (discovers available GGUF models)
            let model_registry = ModelRegistry::init(&data_dir);
            app.manage(model_registry);

            // Initialize worker supervisor and job coordinator
            let worker_path = resolve_worker_binary_path().unwrap_or_else(|e| {
                log::warn!(
                    "Worker binary resolution failed: {}. App will start in degraded mode.",
                    e
                );
                std::path::PathBuf::from("modutone-worker")
            });

            let supervisor = WorkerSupervisor::new(worker_path);
            let coordinator = JobCoordinator::new();

            app.manage(supervisor.clone());
            app.manage(coordinator.clone());

            // Probe platform capabilities (privacy blackout support)
            let capabilities = PlatformCapabilities::probe(app.handle());
            app.manage(capabilities);

            // Spawn worker process in background (non-blocking)
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = supervisor.start(app_handle, coordinator).await {
                    log::warn!("Worker startup failed: {}. Running in degraded mode.", e);
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings::settings_get,
            settings::settings_update,
            settings::model_alias_set,
            settings::model_alias_clear,
            profiles::profiles_list,
            profiles::profiles_create,
            profiles::profiles_update,
            profiles::profiles_delete,
            profiles::profiles_reset_to_default,
            tags::tags_list,
            tags::tags_create,
            tags::tags_update,
            tags::tags_delete,
            models::models_list,
            runtime::runtime_get_status,
            runtime::runtime_warm_model,
            generation::generation_start_initial,
            generation::generation_start_refinement,
            generation::generation_cancel,
            platform::app_set_launch_at_login,
            platform::app_set_tray_enabled,
            platform::app_set_privacy_blackout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ModuTone");
}
