// Phase: 2
// Tag repository: CRUD for custom tags. Built-in tags are read-only and
// loaded from bundled data (builtin_data.rs).
// Path: <data_dir>/tags/custom_tags.json (per installer_migration_spec.md §2.2)
// Missing file → empty custom tags. Corrupt file → backup and return empty.

use std::fs;
use std::path::{Path, PathBuf};

use crate::contracts::errors::IpcError;
use crate::domain::tags::{BuiltInTag, CustomTag};

use super::atomic_writer::atomic_write;
use super::builtin_data;
use super::corruption::{backup_file, BackupReason};

const TAGS_DIR: &str = "tags";
const CUSTOM_TAGS_FILE: &str = "custom_tags.json";

pub struct TagRepository {
    dir_path: PathBuf,
    file_path: PathBuf,
}

impl TagRepository {
    pub fn new(data_dir: &Path) -> Self {
        let dir_path = data_dir.join(TAGS_DIR);
        Self {
            file_path: dir_path.join(CUSTOM_TAGS_FILE),
            dir_path,
        }
    }

    /// Ensure the tags subdirectory exists.
    pub fn ensure_dir(&self) -> Result<(), IpcError> {
        fs::create_dir_all(&self.dir_path).map_err(|e| IpcError {
            code: "TAGS_DIR_CREATE_FAILED".to_string(),
            message: "Failed to create tags directory".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })
    }

    /// Load built-in tags from bundled data (always available, immutable).
    pub fn load_built_in_tags() -> Vec<BuiltInTag> {
        builtin_data::built_in_tags()
    }

    /// Load custom tags from disk. Returns empty vec if file missing or corrupt.
    pub fn load_custom_tags(&self) -> Result<(Vec<CustomTag>, bool), IpcError> {
        if !self.file_path.exists() {
            return Ok((Vec::new(), false));
        }

        let raw = fs::read_to_string(&self.file_path).map_err(|e| IpcError {
            code: "TAGS_READ_FAILED".to_string(),
            message: "Failed to read custom tags file".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;

        match serde_json::from_str::<Vec<CustomTag>>(&raw) {
            Ok(tags) => Ok((tags, false)),
            Err(e) => {
                log::warn!(
                    "Corrupt custom tags file, backing up and using empty: {}",
                    e
                );
                backup_file(&self.file_path, BackupReason::Corrupt)?;
                Ok((Vec::new(), true))
            }
        }
    }

    /// Save custom tags to disk atomically.
    pub fn save_custom_tags(&self, tags: &[CustomTag]) -> Result<(), IpcError> {
        self.ensure_dir()?;
        let json = serde_json::to_string_pretty(tags).map_err(|e| IpcError {
            code: "TAGS_SERIALIZE_FAILED".to_string(),
            message: "Failed to serialize custom tags".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;
        atomic_write(&self.file_path, json.as_bytes())
    }

    /// Validate a tag name. Returns an error if invalid.
    pub fn validate_name(name: &str) -> Result<(), IpcError> {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 50 {
            return Err(IpcError {
                code: "VALIDATION_FAILED".to_string(),
                message: "Tag name must be 1–50 characters".to_string(),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }

    /// Validate a tag instruction body. Returns an error if invalid.
    pub fn validate_instruction_body(body: &str) -> Result<(), IpcError> {
        if body.is_empty() || body.len() > 2_000 {
            return Err(IpcError {
                code: "VALIDATION_FAILED".to_string(),
                message: "Tag instruction body must be 1–2000 characters".to_string(),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }

    /// Check that a tag ID does not refer to a built-in tag.
    pub fn guard_not_built_in(id: &str) -> Result<(), IpcError> {
        if id.starts_with("builtin-") {
            return Err(IpcError {
                code: "CANNOT_EDIT_BUILTIN".to_string(),
                message: "Built-in tags cannot be modified".to_string(),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }

    /// Check for duplicate tag name among existing custom tags.
    pub fn guard_no_duplicate_name(
        name: &str,
        existing: &[CustomTag],
        exclude_id: Option<&str>,
    ) -> Result<(), IpcError> {
        let name_lower = name.trim().to_lowercase();
        let duplicate = existing.iter().any(|t| {
            let dominated = exclude_id.is_some_and(|eid| t.id == eid);
            !dominated && t.name.trim().to_lowercase() == name_lower
        });

        if duplicate {
            return Err(IpcError {
                code: "DUPLICATE_TAG_NAME".to_string(),
                message: format!("A tag named '{}' already exists", name.trim()),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }
}
