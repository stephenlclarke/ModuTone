// Phase: 2
// Comprehensive tests for the persistence layer.

#[cfg(test)]
mod settings_tests {
    use crate::domain::settings::Settings;
    use crate::services::persistence::settings_repo::SettingsRepository;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn missing_file_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let repo = SettingsRepository::new(dir.path());
        let (settings, corrupt) = repo.load().unwrap();
        assert!(!corrupt);
        assert_eq!(settings.schema_version, 1);
        assert!(!settings.tray_enabled);
        assert!(!settings.launch_at_login);
    }

    #[test]
    fn round_trip_through_json() {
        let dir = TempDir::new().unwrap();
        let repo = SettingsRepository::new(dir.path());

        let settings = Settings {
            tray_enabled: true,
            selected_model_id: Some("test-model".to_string()),
            ..Settings::default()
        };
        repo.save(&settings).unwrap();

        let (loaded, corrupt) = repo.load().unwrap();
        assert!(!corrupt);
        assert!(loaded.tray_enabled);
        assert_eq!(loaded.selected_model_id, Some("test-model".to_string()));
    }

    #[test]
    fn corrupt_file_returns_defaults_and_creates_backup() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("settings.json");
        fs::write(&file, "not valid json{{{").unwrap();

        let repo = SettingsRepository::new(dir.path());
        let (settings, corrupt) = repo.load().unwrap();
        assert!(corrupt);
        assert_eq!(settings, Settings::default());

        // Verify backup file was created with .corrupt. naming
        let entries: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .unwrap_or("")
                    .starts_with("settings.json.corrupt.")
            })
            .collect();
        assert_eq!(entries.len(), 1);
    }
}

#[cfg(test)]
mod profile_tests {
    use crate::services::persistence::builtin_data::FACTORY_DEFAULT_PROFILE_ID;
    use crate::services::persistence::profile_repo::ProfileRepository;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn missing_file_returns_factory_default() {
        let dir = TempDir::new().unwrap();
        let repo = ProfileRepository::new(dir.path());
        let (profiles, corrupt) = repo.load().unwrap();
        assert!(!corrupt);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, FACTORY_DEFAULT_PROFILE_ID);
        assert!(profiles[0].is_factory_default);
    }

    #[test]
    fn factory_default_always_present_after_load() {
        let dir = TempDir::new().unwrap();
        let profiles_dir = dir.path().join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();
        let file = profiles_dir.join("profiles.json");
        // Write a file with a custom profile but no factory default
        fs::write(&file, r#"[{"id":"custom-1","name":"Custom","instructionBody":"test","isFactoryDefault":false,"createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-01-01T00:00:00Z"}]"#).unwrap();

        let repo = ProfileRepository::new(dir.path());
        let (profiles, corrupt) = repo.load().unwrap();
        assert!(!corrupt);
        assert!(profiles.iter().any(|p| p.id == FACTORY_DEFAULT_PROFILE_ID));
        assert!(profiles.iter().any(|p| p.id == "custom-1"));
    }

    #[test]
    fn corrupt_file_returns_factory_default_and_creates_backup() {
        let dir = TempDir::new().unwrap();
        let profiles_dir = dir.path().join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();
        let file = profiles_dir.join("profiles.json");
        fs::write(&file, "garbage").unwrap();

        let repo = ProfileRepository::new(dir.path());
        let (profiles, corrupt) = repo.load().unwrap();
        assert!(corrupt);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, FACTORY_DEFAULT_PROFILE_ID);

        // Verify backup with .corrupt. naming in profiles/ subdirectory
        let entries: Vec<_> = fs::read_dir(&profiles_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .unwrap_or("")
                    .starts_with("profiles.json.corrupt.")
            })
            .collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn cannot_delete_factory_default() {
        let result = ProfileRepository::guard_not_factory_default(FACTORY_DEFAULT_PROFILE_ID);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "CANNOT_DELETE_FACTORY_DEFAULT");
    }

    #[test]
    fn can_delete_custom_profile() {
        let result = ProfileRepository::guard_not_factory_default("custom-id");
        assert!(result.is_ok());
    }

    #[test]
    fn reset_guard_accepts_factory_default() {
        let result = ProfileRepository::guard_is_factory_default(FACTORY_DEFAULT_PROFILE_ID);
        assert!(result.is_ok());
    }

    #[test]
    fn reset_guard_rejects_custom_profile() {
        let result = ProfileRepository::guard_is_factory_default("custom-id");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "NOT_FACTORY_DEFAULT");
    }

    #[test]
    fn validate_name_accepts_valid() {
        assert!(ProfileRepository::validate_name("My Profile").is_ok());
        assert!(ProfileRepository::validate_name("A").is_ok());
        assert!(ProfileRepository::validate_name(&"x".repeat(100)).is_ok());
    }

    #[test]
    fn validate_name_rejects_invalid() {
        assert!(ProfileRepository::validate_name("").is_err());
        assert!(ProfileRepository::validate_name("   ").is_err());
        assert!(ProfileRepository::validate_name(&"x".repeat(101)).is_err());
    }

    #[test]
    fn validate_instruction_body_accepts_valid() {
        assert!(ProfileRepository::validate_instruction_body("Rewrite clearly.").is_ok());
        assert!(ProfileRepository::validate_instruction_body(&"x".repeat(10000)).is_ok());
    }

    #[test]
    fn validate_instruction_body_rejects_invalid() {
        assert!(ProfileRepository::validate_instruction_body("").is_err());
        assert!(ProfileRepository::validate_instruction_body(&"x".repeat(10001)).is_err());
    }
}

#[cfg(test)]
mod tag_tests {
    use crate::domain::tags::CustomTag;
    use crate::services::persistence::tag_repo::TagRepository;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn built_in_tags_are_loaded() {
        let tags = TagRepository::load_built_in_tags();
        assert!(!tags.is_empty());
        assert!(tags.iter().all(|t| t.is_built_in));
        assert!(tags.iter().all(|t| t.id.starts_with("builtin-")));
    }

    #[test]
    fn missing_file_returns_empty_custom_tags() {
        let dir = TempDir::new().unwrap();
        let repo = TagRepository::new(dir.path());
        let (tags, corrupt) = repo.load_custom_tags().unwrap();
        assert!(!corrupt);
        assert!(tags.is_empty());
    }

    #[test]
    fn corrupt_file_returns_empty_and_creates_backup() {
        let dir = TempDir::new().unwrap();
        let tags_dir = dir.path().join("tags");
        fs::create_dir_all(&tags_dir).unwrap();
        let file = tags_dir.join("custom_tags.json");
        fs::write(&file, "{{invalid}}").unwrap();

        let repo = TagRepository::new(dir.path());
        let (tags, corrupt) = repo.load_custom_tags().unwrap();
        assert!(corrupt);
        assert!(tags.is_empty());

        // Verify backup with .corrupt. naming in tags/ subdirectory
        let entries: Vec<_> = fs::read_dir(&tags_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .unwrap_or("")
                    .starts_with("custom_tags.json.corrupt.")
            })
            .collect();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn cannot_edit_builtin_tag() {
        let result = TagRepository::guard_not_built_in("builtin-formal");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "CANNOT_EDIT_BUILTIN");
    }

    #[test]
    fn can_edit_custom_tag() {
        let result = TagRepository::guard_not_built_in("custom-tag-id");
        assert!(result.is_ok());
    }

    #[test]
    fn validate_name_accepts_valid() {
        assert!(TagRepository::validate_name("My Tag").is_ok());
        assert!(TagRepository::validate_name(&"x".repeat(50)).is_ok());
    }

    #[test]
    fn validate_name_rejects_invalid() {
        assert!(TagRepository::validate_name("").is_err());
        assert!(TagRepository::validate_name("   ").is_err());
        assert!(TagRepository::validate_name(&"x".repeat(51)).is_err());
    }

    #[test]
    fn validate_instruction_body_accepts_valid() {
        assert!(TagRepository::validate_instruction_body("Be concise.").is_ok());
        assert!(TagRepository::validate_instruction_body(&"x".repeat(2000)).is_ok());
    }

    #[test]
    fn validate_instruction_body_rejects_invalid() {
        assert!(TagRepository::validate_instruction_body("").is_err());
        assert!(TagRepository::validate_instruction_body(&"x".repeat(2001)).is_err());
    }

    #[test]
    fn duplicate_name_detected() {
        let existing = vec![CustomTag {
            id: "t1".to_string(),
            name: "Existing Tag".to_string(),
            category: crate::contracts::shared::TagCategory::Other,
            instruction_body: "test".to_string(),
            is_built_in: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        let result = TagRepository::guard_no_duplicate_name("existing tag", &existing, None);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "DUPLICATE_TAG_NAME");
    }

    #[test]
    fn duplicate_name_excluded_for_self_update() {
        let existing = vec![CustomTag {
            id: "t1".to_string(),
            name: "Existing Tag".to_string(),
            category: crate::contracts::shared::TagCategory::Other,
            instruction_body: "test".to_string(),
            is_built_in: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }];

        // When updating t1 itself, it should not conflict with itself
        let result = TagRepository::guard_no_duplicate_name("Existing Tag", &existing, Some("t1"));
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod metadata_store_tests {
    use crate::services::persistence::metadata_store::MetadataStore;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn init_creates_data_dir_and_subdirs() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("modutone-test");
        let store = MetadataStore::init(&sub).unwrap();

        assert!(sub.exists());
        assert!(sub.join("profiles").exists());
        assert!(sub.join("tags").exists());
        assert!(sub.join("migration").exists());
        assert!(!store.is_read_only());

        // Settings should be defaults
        let settings = store.get_settings().unwrap();
        assert_eq!(settings.schema_version, 1);

        // Profiles should have factory default
        let profiles = store.list_profiles().unwrap();
        assert_eq!(profiles.len(), 1);
        assert!(profiles[0].is_factory_default);

        // Tags should have built-in tags and no custom tags
        let (built_in, custom) = store.list_tags().unwrap();
        assert!(!built_in.is_empty());
        assert!(custom.is_empty());

        // Migration log should exist
        assert!(sub.join("migration").join("migration_log.json").exists());
    }

    #[test]
    fn settings_update_persists_to_disk() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        let req = crate::contracts::commands::SettingsUpdateRequest {
            contract_version: 1,
            theme_preference: Some(crate::contracts::shared::ThemePreference::Dark),
            tray_enabled: Some(true),
            launch_at_login: None,
            privacy_blackout_enabled: None,
            selected_model_id: None,
            last_selected_profile_id: None,
            last_successful_model_id: None,
            visual_style: None,
            motion_preference: None,
        };
        store.update_settings(&req).unwrap();

        // Reload from disk
        let store2 = MetadataStore::init(dir.path()).unwrap();
        let settings = store2.get_settings().unwrap();
        assert_eq!(
            settings.theme_preference,
            crate::contracts::shared::ThemePreference::Dark
        );
        assert!(settings.tray_enabled);
    }

    #[test]
    fn profile_crud_round_trip() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        // Create
        let resp = store
            .create_profile(&crate::contracts::commands::ProfileCreateRequest {
                contract_version: 1,
                name: "Test Profile".to_string(),
                instruction_body: "Make it better.".to_string(),
            })
            .unwrap();
        let id = resp.id.clone();

        // Verify file is in profiles/ subdirectory
        assert!(dir.path().join("profiles").join("profiles.json").exists());

        // List
        let profiles = store.list_profiles().unwrap();
        assert_eq!(profiles.len(), 2); // factory default + new
        assert!(profiles.iter().any(|p| p.id == id));

        // Update
        store
            .update_profile(&crate::contracts::commands::ProfileUpdateRequest {
                contract_version: 1,
                id: id.clone(),
                name: Some("Updated Name".to_string()),
                instruction_body: None,
            })
            .unwrap();
        let profiles = store.list_profiles().unwrap();
        let updated = profiles.iter().find(|p| p.id == id).unwrap();
        assert_eq!(updated.name, "Updated Name");

        // Delete
        store.delete_profile(&id).unwrap();
        let profiles = store.list_profiles().unwrap();
        assert_eq!(profiles.len(), 1);
    }

    #[test]
    fn factory_default_cannot_be_deleted() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        let result = store.delete_profile("factory-default");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "CANNOT_DELETE_FACTORY_DEFAULT");
    }

    #[test]
    fn factory_default_cannot_be_updated() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        let result = store.update_profile(&crate::contracts::commands::ProfileUpdateRequest {
            contract_version: 1,
            id: "factory-default".to_string(),
            name: None,
            instruction_body: Some("Modified instruction.".to_string()),
        });
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "FACTORY_DEFAULT_IMMUTABLE");
    }

    #[test]
    fn factory_default_can_be_reset() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        // Reset (even without prior modification) should succeed
        store.reset_profile_to_default("factory-default").unwrap();
        let profile = store.get_profile_by_id("factory-default").unwrap();
        assert!(profile
            .instruction_body
            .contains("local text-rewrite engine"));
    }

    #[test]
    fn list_profiles_redacts_factory_default_instruction_body() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        // Create a custom profile
        store
            .create_profile(&crate::contracts::commands::ProfileCreateRequest {
                contract_version: 1,
                name: "Custom".to_string(),
                instruction_body: "Custom body.".to_string(),
            })
            .unwrap();

        let profiles = store.list_profiles().unwrap();

        // Factory default should have empty instruction_body in IPC response
        let fd = profiles.iter().find(|p| p.is_factory_default).unwrap();
        assert!(fd.instruction_body.is_empty());

        // Custom profile should preserve instruction_body
        let custom = profiles.iter().find(|p| !p.is_factory_default).unwrap();
        assert_eq!(custom.instruction_body, "Custom body.");
    }

    #[test]
    fn tag_crud_round_trip() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        // Create
        let resp = store
            .create_tag(&crate::contracts::commands::TagCreateRequest {
                contract_version: 1,
                name: "Custom Tag".to_string(),
                category: crate::contracts::shared::TagCategory::Other,
                instruction_body: "Custom instruction.".to_string(),
            })
            .unwrap();
        let id = resp.id.clone();

        // Verify file is in tags/ subdirectory
        assert!(dir.path().join("tags").join("custom_tags.json").exists());

        // List
        let (built_in, custom) = store.list_tags().unwrap();
        assert!(!built_in.is_empty());
        assert_eq!(custom.len(), 1);
        assert_eq!(custom[0].id, id);

        // Update
        store
            .update_tag(&crate::contracts::commands::TagUpdateRequest {
                contract_version: 1,
                id: id.clone(),
                name: Some("Renamed Tag".to_string()),
                category: None,
                instruction_body: None,
            })
            .unwrap();
        let (_, custom) = store.list_tags().unwrap();
        assert_eq!(custom[0].name, "Renamed Tag");

        // Delete
        store.delete_tag(&id).unwrap();
        let (_, custom) = store.list_tags().unwrap();
        assert!(custom.is_empty());
    }

    #[test]
    fn builtin_tag_cannot_be_edited() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        let result = store.update_tag(&crate::contracts::commands::TagUpdateRequest {
            contract_version: 1,
            id: "builtin-formal".to_string(),
            name: Some("Hacked".to_string()),
            category: None,
            instruction_body: None,
        });
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "CANNOT_EDIT_BUILTIN");
    }

    #[test]
    fn builtin_tag_cannot_be_deleted() {
        let dir = TempDir::new().unwrap();
        let store = MetadataStore::init(dir.path()).unwrap();

        let result = store.delete_tag("builtin-formal");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "CANNOT_EDIT_BUILTIN");
    }

    #[test]
    fn corruption_recovery_loads_defaults_and_backs_up() {
        let dir = TempDir::new().unwrap();
        // Write corrupt files in the correct subdirectory locations
        fs::write(dir.path().join("settings.json"), "corrupt!!!").unwrap();
        let profiles_dir = dir.path().join("profiles");
        fs::create_dir_all(&profiles_dir).unwrap();
        fs::write(profiles_dir.join("profiles.json"), "corrupt!!!").unwrap();
        let tags_dir = dir.path().join("tags");
        fs::create_dir_all(&tags_dir).unwrap();
        fs::write(tags_dir.join("custom_tags.json"), "corrupt!!!").unwrap();

        let store = MetadataStore::init(dir.path()).unwrap();
        assert!(store.was_corruption_detected());

        // Defaults should be loaded
        let settings = store.get_settings().unwrap();
        assert_eq!(settings.schema_version, 1);
        let profiles = store.list_profiles().unwrap();
        assert_eq!(profiles.len(), 1);
        let (_, custom) = store.list_tags().unwrap();
        assert!(custom.is_empty());

        // Backup files should exist with .corrupt. naming
        let settings_backups: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().unwrap_or("").contains(".corrupt."))
            .collect();
        assert_eq!(settings_backups.len(), 1);

        let profile_backups: Vec<_> = fs::read_dir(&profiles_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().unwrap_or("").contains(".corrupt."))
            .collect();
        assert_eq!(profile_backups.len(), 1);

        let tag_backups: Vec<_> = fs::read_dir(&tags_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_str().unwrap_or("").contains(".corrupt."))
            .collect();
        assert_eq!(tag_backups.len(), 1);
    }

    #[test]
    fn data_persists_across_store_instances() {
        let dir = TempDir::new().unwrap();

        // Write data
        {
            let store = MetadataStore::init(dir.path()).unwrap();
            store
                .create_profile(&crate::contracts::commands::ProfileCreateRequest {
                    contract_version: 1,
                    name: "Persistent Profile".to_string(),
                    instruction_body: "Persists across restarts.".to_string(),
                })
                .unwrap();
            store
                .create_tag(&crate::contracts::commands::TagCreateRequest {
                    contract_version: 1,
                    name: "Persistent Tag".to_string(),
                    category: crate::contracts::shared::TagCategory::Tone,
                    instruction_body: "A tone tag.".to_string(),
                })
                .unwrap();
        }

        // Read data in new instance
        {
            let store = MetadataStore::init(dir.path()).unwrap();
            let profiles = store.list_profiles().unwrap();
            assert_eq!(profiles.len(), 2);
            assert!(profiles.iter().any(|p| p.name == "Persistent Profile"));

            let (_, custom) = store.list_tags().unwrap();
            assert_eq!(custom.len(), 1);
            assert_eq!(custom[0].name, "Persistent Tag");
        }
    }

    #[test]
    fn migration_log_records_events() {
        let dir = TempDir::new().unwrap();
        let _store = MetadataStore::init(dir.path()).unwrap();

        let log_path = dir.path().join("migration").join("migration_log.json");
        assert!(log_path.exists());

        let raw = fs::read_to_string(&log_path).unwrap();
        // Should contain at least one entry
        assert!(raw.contains("schemaVersion"));
        assert!(raw.contains("FRESH_INSTALL") || raw.contains("NORMAL_STARTUP"));
    }
}

/// Privacy scan: verify that persisted file formats contain only metadata fields,
/// never user content (input text, output text, refinement instructions).
#[cfg(test)]
mod privacy_tests {
    use crate::contracts::shared::TagCategory;
    use crate::domain::profiles::PromptProfile;
    use crate::domain::settings::Settings;
    use crate::domain::tags::CustomTag;

    /// Verify the Settings struct has no content-bearing fields.
    #[test]
    fn settings_has_no_content_fields() {
        let s = Settings::default();
        let json = serde_json::to_string_pretty(&s).unwrap();
        // Settings should contain only metadata keys
        assert!(json.contains("schemaVersion"));
        assert!(json.contains("themePreference"));
        assert!(json.contains("trayEnabled"));
        // Should NOT contain any content-related keys
        assert!(!json.contains("inputText"));
        assert!(!json.contains("outputText"));
        assert!(!json.contains("sourceText"));
        assert!(!json.contains("acceptedOutput"));
        assert!(!json.contains("refinementInstruction"));
        assert!(!json.contains("proposedOutput"));
        assert!(!json.contains("partialText"));
    }

    /// Verify the PromptProfile struct contains only configuration metadata.
    #[test]
    fn profile_has_no_user_content_fields() {
        let p = PromptProfile {
            id: "test".to_string(),
            name: "Test".to_string(),
            instruction_body: "Rewrite clearly.".to_string(),
            is_factory_default: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string_pretty(&p).unwrap();
        // instructionBody is a prompt template (configuration), not user content
        assert!(json.contains("instructionBody"));
        // Should NOT contain any user content keys
        assert!(!json.contains("inputText"));
        assert!(!json.contains("outputText"));
        assert!(!json.contains("sourceText"));
        assert!(!json.contains("acceptedOutput"));
        assert!(!json.contains("refinementInstruction"));
    }

    /// Verify the CustomTag struct contains only configuration metadata.
    #[test]
    fn custom_tag_has_no_user_content_fields() {
        let t = CustomTag {
            id: "test".to_string(),
            name: "Test".to_string(),
            category: TagCategory::Other,
            instruction_body: "Be concise.".to_string(),
            is_built_in: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string_pretty(&t).unwrap();
        assert!(json.contains("instructionBody"));
        assert!(!json.contains("inputText"));
        assert!(!json.contains("outputText"));
        assert!(!json.contains("sourceText"));
        assert!(!json.contains("acceptedOutput"));
        assert!(!json.contains("refinementInstruction"));
    }
}
