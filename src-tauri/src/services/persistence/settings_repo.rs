// Phase: 2
// Settings repository: load, save, validate settings from/to JSON file.
// Path: <data_dir>/settings.json (flat, per installer_migration_spec.md §2.2)
// Missing file → return defaults. Corrupt file → backup and return defaults.

use std::fs;
use std::path::{Path, PathBuf};

use crate::contracts::errors::IpcError;
use crate::domain::settings::Settings;

use super::atomic_writer::atomic_write;
use super::corruption::{backup_file, BackupReason};

const SETTINGS_FILE: &str = "settings.json";

pub struct SettingsRepository {
    file_path: PathBuf,
}

impl SettingsRepository {
    pub fn new(data_dir: &Path) -> Self {
        Self {
            file_path: data_dir.join(SETTINGS_FILE),
        }
    }

    /// Load settings from disk. Returns defaults if file missing or corrupt.
    /// On corruption, backs up the corrupt file before returning defaults.
    pub fn load(&self) -> Result<(Settings, bool), IpcError> {
        if !self.file_path.exists() {
            return Ok((Settings::default(), false));
        }

        let raw = fs::read_to_string(&self.file_path).map_err(|e| IpcError {
            code: "SETTINGS_READ_FAILED".to_string(),
            message: "Failed to read settings file".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;

        match serde_json::from_str::<Settings>(&raw) {
            Ok(settings) => Ok((settings, false)),
            Err(e) => {
                log::warn!(
                    "Corrupt settings file, backing up and using defaults: {}",
                    e
                );
                backup_file(&self.file_path, BackupReason::Corrupt)?;
                Ok((Settings::default(), true))
            }
        }
    }

    /// Save settings to disk atomically.
    pub fn save(&self, settings: &Settings) -> Result<(), IpcError> {
        let json = serde_json::to_string_pretty(settings).map_err(|e| IpcError {
            code: "SETTINGS_SERIALIZE_FAILED".to_string(),
            message: "Failed to serialize settings".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;
        atomic_write(&self.file_path, json.as_bytes())
    }
}
