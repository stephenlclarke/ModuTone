// Phase: 10
// Upgrade verification test (P10-2)
//
// Verifies that custom data survives across MetadataStore reinitializations,
// simulating an app upgrade that preserves the data directory.

use tempfile::TempDir;

use modutone_app::contracts::commands::{
    ProfileCreateRequest, SettingsUpdateRequest, TagCreateRequest,
};
use modutone_app::contracts::shared::{TagCategory, ThemePreference};
use modutone_app::services::persistence::metadata_store::MetadataStore;

#[test]
fn custom_data_survives_reinit() {
    let dir = TempDir::new().unwrap();

    let custom_profile_id;
    let custom_tag_id;

    // Phase 1: Initialize store and add custom data
    {
        let store = MetadataStore::init(dir.path()).unwrap();

        // Modify settings (this writes settings.json to disk)
        store
            .update_settings(&SettingsUpdateRequest {
                contract_version: 1,
                theme_preference: Some(ThemePreference::Dark),
                tray_enabled: Some(true),
                launch_at_login: None,
                privacy_blackout_enabled: None,
                selected_model_id: Some(Some("test-model".to_string())),
                last_selected_profile_id: None,
                last_successful_model_id: None,
                visual_style: None,
                motion_preference: None,
            })
            .unwrap();

        // Create custom profile
        let resp = store
            .create_profile(&ProfileCreateRequest {
                contract_version: 1,
                name: "Upgrade Test Profile".to_string(),
                instruction_body: "Rewrite for testing upgrades.".to_string(),
            })
            .unwrap();
        custom_profile_id = resp.id;

        // Create custom tag
        let resp = store
            .create_tag(&TagCreateRequest {
                contract_version: 1,
                name: "Upgrade Test Tag".to_string(),
                category: TagCategory::Tone,
                instruction_body: "Formal academic tone.".to_string(),
            })
            .unwrap();
        custom_tag_id = resp.id;

        // Verify data exists
        assert_eq!(store.list_profiles().unwrap().len(), 2); // factory + custom
        assert_eq!(store.list_tags().unwrap().1.len(), 1); // 1 custom tag
    }

    // Phase 2: Drop store and reinitialize from same directory (simulates app restart)
    {
        let store = MetadataStore::init(dir.path()).unwrap();

        // Settings survived
        let settings = store.get_settings().unwrap();
        assert_eq!(settings.theme_preference, ThemePreference::Dark);
        assert!(settings.tray_enabled);
        assert_eq!(settings.selected_model_id, Some("test-model".to_string()));

        // Custom profile survived
        let profiles = store.list_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
        let custom = profiles.iter().find(|p| p.id == custom_profile_id).unwrap();
        assert_eq!(custom.name, "Upgrade Test Profile");
        assert_eq!(custom.instruction_body, "Rewrite for testing upgrades.");

        // Custom tag survived
        let (built_in, custom_tags) = store.list_tags().unwrap();
        assert!(!built_in.is_empty()); // built-in tags present
        assert_eq!(custom_tags.len(), 1);
        assert_eq!(custom_tags[0].id, custom_tag_id);
        assert_eq!(custom_tags[0].name, "Upgrade Test Tag");

        // Factory default profile still present
        assert!(profiles.iter().any(|p| p.is_factory_default));
    }
}

#[test]
fn future_version_triggers_downgrade_recovery() {
    let dir = TempDir::new().unwrap();

    // Phase 1: Create store with custom data and force settings to disk
    {
        let store = MetadataStore::init(dir.path()).unwrap();
        // Write settings so settings.json exists on disk
        store
            .update_settings(&SettingsUpdateRequest {
                contract_version: 1,
                theme_preference: Some(ThemePreference::Light),
                tray_enabled: None,
                launch_at_login: None,
                privacy_blackout_enabled: None,
                selected_model_id: None,
                last_selected_profile_id: None,
                last_successful_model_id: None,
                visual_style: None,
                motion_preference: None,
            })
            .unwrap();
        store
            .create_profile(&ProfileCreateRequest {
                contract_version: 1,
                name: "Will Be Preserved".to_string(),
                instruction_body: "This profile will survive downgrade recovery.".to_string(),
            })
            .unwrap();
    }

    // Phase 2: Tamper settings to simulate a future version
    {
        let settings_path = dir.path().join("settings.json");
        let raw = std::fs::read_to_string(&settings_path).unwrap();
        let tampered = raw.replace("\"schemaVersion\": 1", "\"schemaVersion\": 99");
        std::fs::write(&settings_path, tampered).unwrap();
    }

    // Phase 3: Reinitialize — should trigger downgrade recovery
    {
        let store = MetadataStore::init(dir.path()).unwrap();

        // Settings should be reset to defaults
        let settings = store.get_settings().unwrap();
        assert_eq!(settings.schema_version, 1);
        assert!(!settings.tray_enabled); // default

        // Profiles should survive (they have their own file)
        let profiles = store.list_profiles().unwrap();
        assert_eq!(profiles.len(), 2); // factory + custom

        // Backup file should exist
        let backups: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().unwrap_or("").contains(".future."))
            .collect();
        assert!(
            !backups.is_empty(),
            "Backup with .future. suffix should exist"
        );

        // Migration log should contain DOWNGRADE_RECOVERY
        let log_path = dir.path().join("migration").join("migration_log.json");
        let log_content = std::fs::read_to_string(log_path).unwrap();
        assert!(
            log_content.contains("DOWNGRADE_RECOVERY"),
            "Migration log should contain DOWNGRADE_RECOVERY event"
        );
    }
}

#[test]
fn downgrade_recovery_logs_event() {
    let dir = TempDir::new().unwrap();

    // Initialize normally and write settings to disk
    {
        let store = MetadataStore::init(dir.path()).unwrap();
        store
            .update_settings(&SettingsUpdateRequest {
                contract_version: 1,
                theme_preference: Some(ThemePreference::Light),
                tray_enabled: None,
                launch_at_login: None,
                privacy_blackout_enabled: None,
                selected_model_id: None,
                last_selected_profile_id: None,
                last_successful_model_id: None,
                visual_style: None,
                motion_preference: None,
            })
            .unwrap();
    }

    // Tamper to future version
    {
        let settings_path = dir.path().join("settings.json");
        let raw = std::fs::read_to_string(&settings_path).unwrap();
        let tampered = raw.replace("\"schemaVersion\": 1", "\"schemaVersion\": 42");
        std::fs::write(&settings_path, tampered).unwrap();
    }

    // Reinitialize
    {
        let _store = MetadataStore::init(dir.path()).unwrap();
    }

    // Check migration log
    let log_path = dir.path().join("migration").join("migration_log.json");
    let log_content = std::fs::read_to_string(log_path).unwrap();
    assert!(
        log_content.contains("DOWNGRADE_RECOVERY"),
        "Migration log must contain DOWNGRADE_RECOVERY"
    );
    assert!(
        log_content.contains("42"),
        "Migration log should reference future version 42"
    );
}
