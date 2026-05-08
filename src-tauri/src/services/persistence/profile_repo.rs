// Phase: 2
// Profile repository: CRUD for prompt profiles with factory default protection.
// Path: <data_dir>/profiles/profiles.json (per installer_migration_spec.md §2.2)
// Missing file → factory default only. Corrupt file → backup and return default.

use std::fs;
use std::path::{Path, PathBuf};

use crate::contracts::errors::IpcError;
use crate::domain::profiles::PromptProfile;

use super::atomic_writer::atomic_write;
use super::builtin_data::{
    factory_default_instruction_body, factory_default_profile, FACTORY_DEFAULT_PROFILE_ID,
};
use super::corruption::{backup_file, BackupReason};

const PROFILES_DIR: &str = "profiles";
const PROFILES_FILE: &str = "profiles.json";

pub struct ProfileRepository {
    dir_path: PathBuf,
    file_path: PathBuf,
}

impl ProfileRepository {
    pub fn new(data_dir: &Path) -> Self {
        let dir_path = data_dir.join(PROFILES_DIR);
        Self {
            file_path: dir_path.join(PROFILES_FILE),
            dir_path,
        }
    }

    /// Ensure the profiles subdirectory exists.
    pub fn ensure_dir(&self) -> Result<(), IpcError> {
        fs::create_dir_all(&self.dir_path).map_err(|e| IpcError {
            code: "PROFILES_DIR_CREATE_FAILED".to_string(),
            message: "Failed to create profiles directory".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })
    }

    /// Load profiles from disk. Returns [factory_default] if file missing or corrupt.
    /// Ensures factory default profile is always present.
    pub fn load(&self) -> Result<(Vec<PromptProfile>, bool), IpcError> {
        if !self.file_path.exists() {
            return Ok((vec![factory_default_profile()], false));
        }

        let raw = fs::read_to_string(&self.file_path).map_err(|e| IpcError {
            code: "PROFILES_READ_FAILED".to_string(),
            message: "Failed to read profiles file".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;

        match serde_json::from_str::<Vec<PromptProfile>>(&raw) {
            Ok(mut profiles) => {
                // Ensure factory default is always present
                if !profiles.iter().any(|p| p.id == FACTORY_DEFAULT_PROFILE_ID) {
                    profiles.insert(0, factory_default_profile());
                }
                Ok((profiles, false))
            }
            Err(e) => {
                log::warn!(
                    "Corrupt profiles file, backing up and using defaults: {}",
                    e
                );
                backup_file(&self.file_path, BackupReason::Corrupt)?;
                Ok((vec![factory_default_profile()], true))
            }
        }
    }

    /// Save all profiles to disk atomically.
    pub fn save(&self, profiles: &[PromptProfile]) -> Result<(), IpcError> {
        self.ensure_dir()?;
        let json = serde_json::to_string_pretty(profiles).map_err(|e| IpcError {
            code: "PROFILES_SERIALIZE_FAILED".to_string(),
            message: "Failed to serialize profiles".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;
        atomic_write(&self.file_path, json.as_bytes())
    }

    /// Validate a profile name. Returns an error if invalid.
    pub fn validate_name(name: &str) -> Result<(), IpcError> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 100 {
            return Err(IpcError {
                code: "VALIDATION_FAILED".to_string(),
                message: "Profile name must be 1–100 characters".to_string(),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }

    /// Validate a profile instruction body. Returns an error if invalid.
    pub fn validate_instruction_body(body: &str) -> Result<(), IpcError> {
        if body.is_empty() || body.len() > 10_000 {
            return Err(IpcError {
                code: "VALIDATION_FAILED".to_string(),
                message: "Profile instruction body must be 1–10000 characters".to_string(),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }

    /// Check that a profile is not the factory default (for delete guard).
    pub fn guard_not_factory_default(id: &str) -> Result<(), IpcError> {
        if id == FACTORY_DEFAULT_PROFILE_ID {
            return Err(IpcError {
                code: "CANNOT_DELETE_FACTORY_DEFAULT".to_string(),
                message: "The factory default profile cannot be deleted".to_string(),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }

    /// Check that a profile is the factory default (for reset guard).
    pub fn guard_is_factory_default(id: &str) -> Result<(), IpcError> {
        if id != FACTORY_DEFAULT_PROFILE_ID {
            return Err(IpcError {
                code: "NOT_FACTORY_DEFAULT".to_string(),
                message: "Only the factory default profile can be reset".to_string(),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }

    /// Returns the bundled factory default instruction body for reset operations.
    pub fn default_instruction_body() -> &'static str {
        factory_default_instruction_body()
    }
}
