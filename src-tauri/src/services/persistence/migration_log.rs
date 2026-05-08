// Phase: 2
// Migration log: records startup migration events.
// Path: <data_dir>/migration/migration_log.json
// Per installer_migration_spec.md §2.2 and §5 step 3.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::contracts::errors::IpcError;

use super::atomic_writer::atomic_write;

const MIGRATION_DIR: &str = "migration";
const MIGRATION_LOG_FILE: &str = "migration_log.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationLogEntry {
    pub timestamp: String,
    pub event: MigrationEvent,
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MigrationEvent {
    FreshInstall,
    NormalStartup,
    CorruptionRecovery,
    DowngradeRecovery,
    MigrationSuccess,
    MigrationFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MigrationLog {
    entries: Vec<MigrationLogEntry>,
}

pub struct MigrationLogger {
    dir_path: PathBuf,
    file_path: PathBuf,
}

impl MigrationLogger {
    pub fn new(data_dir: &Path) -> Self {
        let dir_path = data_dir.join(MIGRATION_DIR);
        Self {
            file_path: dir_path.join(MIGRATION_LOG_FILE),
            dir_path,
        }
    }

    /// Append an entry to the migration log.
    pub fn log_event(&self, event: MigrationEvent, schema_version: u32, detail: Option<String>) {
        let entry = MigrationLogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            event,
            schema_version,
            detail,
        };

        // Best-effort: don't fail startup if log write fails
        if let Err(e) = self.append_entry(&entry) {
            log::warn!("Failed to write migration log entry: {}", e);
        }
    }

    fn append_entry(&self, entry: &MigrationLogEntry) -> Result<(), IpcError> {
        fs::create_dir_all(&self.dir_path).map_err(|e| IpcError {
            code: "MIGRATION_DIR_CREATE_FAILED".to_string(),
            message: "Failed to create migration directory".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;

        let mut log = if self.file_path.exists() {
            let raw = fs::read_to_string(&self.file_path).unwrap_or_default();
            serde_json::from_str::<MigrationLog>(&raw).unwrap_or(MigrationLog {
                entries: Vec::new(),
            })
        } else {
            MigrationLog {
                entries: Vec::new(),
            }
        };

        log.entries.push(entry.clone());

        // Cap log at 100 entries to prevent unbounded growth
        if log.entries.len() > 100 {
            let drain_count = log.entries.len() - 100;
            log.entries.drain(..drain_count);
        }

        let json = serde_json::to_string_pretty(&log).map_err(|e| IpcError {
            code: "MIGRATION_LOG_SERIALIZE_FAILED".to_string(),
            message: "Failed to serialize migration log".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;

        atomic_write(&self.file_path, json.as_bytes())
    }
}
