// Phase: 10
// Schema migration service.
// Currently supports schema v1. Provides a migration chain for future versions.
// Downgrade recovery: future versions produce DowngradeRecovered instead of crashing.

use crate::contracts::errors::IpcError;

/// Current schema version for all persistent metadata files.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Outcome of a migration check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// Schema is already at the current version.
    Current,
    /// Successfully migrated from an older version.
    Migrated { from: u32, to: u32 },
    /// A future version was detected; caller should reset to defaults.
    DowngradeRecovered { future_version: u32 },
}

pub struct MigrationService;

impl MigrationService {
    /// Check if the loaded schema version is current. If it's an older version,
    /// run migrations. If it's a future version, return DowngradeRecovered.
    pub fn check_and_migrate(loaded_version: u32) -> Result<MigrationOutcome, IpcError> {
        if loaded_version == CURRENT_SCHEMA_VERSION {
            return Ok(MigrationOutcome::Current);
        }

        if loaded_version > CURRENT_SCHEMA_VERSION {
            return Ok(MigrationOutcome::DowngradeRecovered {
                future_version: loaded_version,
            });
        }

        // Run migration chain: v(loaded) → v(loaded+1) → ... → v(current)
        let from = loaded_version;
        let mut version = loaded_version;
        while version < CURRENT_SCHEMA_VERSION {
            version = migrate_one_step(version)?;
        }

        Ok(MigrationOutcome::Migrated { from, to: version })
    }
}

/// Migrate from version N to version N+1.
/// Currently only v1 exists, so this is a stub for future migrations.
fn migrate_one_step(from_version: u32) -> Result<u32, IpcError> {
    // When v2 is introduced, add: 1 => { /* migrate v1 → v2 */ Ok(2) }
    // For now, all past versions are unsupported since we start at v1.
    Err(IpcError {
        code: "MIGRATION_UNSUPPORTED".to_string(),
        message: format!("No migration path from schema version {}", from_version),
        detail: None,
        subsystem: "persistence".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_version_passes() {
        let result = MigrationService::check_and_migrate(CURRENT_SCHEMA_VERSION);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), MigrationOutcome::Current);
    }

    #[test]
    fn future_version_triggers_downgrade_recovery() {
        let result = MigrationService::check_and_migrate(CURRENT_SCHEMA_VERSION + 1);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            MigrationOutcome::DowngradeRecovered {
                future_version: CURRENT_SCHEMA_VERSION + 1
            }
        );
    }

    #[test]
    fn far_future_version_triggers_downgrade_recovery() {
        let result = MigrationService::check_and_migrate(99);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            MigrationOutcome::DowngradeRecovered { future_version: 99 }
        );
    }

    #[test]
    fn unknown_past_version_fails() {
        // Version 0 has no migration path
        let result = MigrationService::check_and_migrate(0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, "MIGRATION_UNSUPPORTED");
    }
}
