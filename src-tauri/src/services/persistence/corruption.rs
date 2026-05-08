// Phase: 2
// Corruption recovery: backup corrupt/incompatible files before replacing with defaults.
// Naming conventions per installer_migration_spec.md §5:
//   - Corrupt/unreadable file → {filename}.corrupt.{timestamp}
//   - Future schema version  → {filename}.future.{timestamp}
//   - Failed migration       → {filename}.migration_failed.{timestamp}

use std::fs;
use std::path::Path;

use crate::contracts::errors::IpcError;

/// Backup reason determines the suffix used in the backup filename.
pub enum BackupReason {
    Corrupt,
    FutureVersion,
    MigrationFailed,
}

impl BackupReason {
    fn suffix(&self) -> &'static str {
        match self {
            BackupReason::Corrupt => "corrupt",
            BackupReason::FutureVersion => "future",
            BackupReason::MigrationFailed => "migration_failed",
        }
    }
}

/// Rename a file to `<name>.<reason>.<timestamp>` so it is preserved
/// for debugging but no longer loaded as the active file.
pub fn backup_file(path: &Path, reason: BackupReason) -> Result<(), IpcError> {
    if !path.exists() {
        return Ok(());
    }

    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let backup_name = format!("{}.{}.{}", file_name, reason.suffix(), timestamp);
    let backup_path = path.with_file_name(backup_name);

    fs::rename(path, &backup_path).map_err(|e| IpcError {
        code: "BACKUP_FAILED".to_string(),
        message: "Failed to back up file".to_string(),
        detail: Some(e.to_string()),
        subsystem: "persistence".to_string(),
    })?;

    log::info!("Backed up file to {:?}", backup_path);
    Ok(())
}
