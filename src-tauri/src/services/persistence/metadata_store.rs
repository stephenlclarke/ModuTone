// Phase: 2
// MetadataStore: top-level managed state for all persistent metadata.
// Holds in-memory caches of settings, profiles, and custom tags.
// Provides thread-safe access via RwLock for concurrent Tauri commands.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

use crate::contracts::commands::{
    BuiltInTagEntry, CustomTagEntry, ProfileCreateRequest, ProfileCreateResponse, ProfileEntry,
    ProfileUpdateRequest, SettingsGetResponse, SettingsUpdateRequest, TagCreateRequest,
    TagCreateResponse, TagUpdateRequest,
};
use crate::contracts::errors::IpcError;
use crate::domain::profiles::PromptProfile;
use crate::domain::settings::Settings;
use crate::domain::tags::{BuiltInTag, CustomTag};

use super::corruption::{backup_file, BackupReason};
use super::migration::{MigrationOutcome, MigrationService};
use super::migration_log::{MigrationEvent, MigrationLogger};
use super::profile_repo::ProfileRepository;
use super::settings_repo::SettingsRepository;
use super::tag_repo::TagRepository;

pub struct MetadataStore {
    settings_repo: SettingsRepository,
    profile_repo: ProfileRepository,
    tag_repo: TagRepository,
    settings: RwLock<Settings>,
    profiles: RwLock<Vec<PromptProfile>>,
    custom_tags: RwLock<Vec<CustomTag>>,
    built_in_tags: Vec<BuiltInTag>,
    read_only: AtomicBool,
    corruption_detected: AtomicBool,
}

impl MetadataStore {
    /// Initialize the store from the given data directory.
    /// Creates the directory if it doesn't exist.
    /// Loads all metadata from disk, applying corruption recovery as needed.
    pub fn init(data_dir: &Path) -> Result<Self, IpcError> {
        std::fs::create_dir_all(data_dir).map_err(|e| IpcError {
            code: "DATA_DIR_CREATE_FAILED".to_string(),
            message: "Failed to create data directory".to_string(),
            detail: Some(e.to_string()),
            subsystem: "persistence".to_string(),
        })?;

        let settings_repo = SettingsRepository::new(data_dir);
        let profile_repo = ProfileRepository::new(data_dir);
        let tag_repo = TagRepository::new(data_dir);
        let migration_logger = MigrationLogger::new(data_dir);

        // Ensure subdirectories exist for profiles/ and tags/
        profile_repo.ensure_dir()?;
        tag_repo.ensure_dir()?;

        let (settings, settings_corrupt) = settings_repo.load()?;
        let (profiles, profiles_corrupt) = profile_repo.load()?;
        let (custom_tags, tags_corrupt) = tag_repo.load_custom_tags()?;
        let built_in_tags = TagRepository::load_built_in_tags();

        // Check schema version and run migrations if needed
        let mut downgrade_recovered = false;
        let (settings, settings_corrupt) =
            match MigrationService::check_and_migrate(settings.schema_version)? {
                MigrationOutcome::Current | MigrationOutcome::Migrated { .. } => {
                    (settings, settings_corrupt)
                }
                MigrationOutcome::DowngradeRecovered { future_version } => {
                    // Backup the future-version settings file
                    let settings_path = data_dir.join("settings.json");
                    let _ = backup_file(&settings_path, BackupReason::FutureVersion);

                    // Reset to defaults
                    let defaults = Settings::default();
                    settings_repo.save(&defaults)?;

                    log::warn!(
                        "Downgrade recovery: schema version {} is from a newer app version. \
                     Settings backed up and reset to defaults.",
                        future_version
                    );

                    migration_logger.log_event(
                        MigrationEvent::DowngradeRecovery,
                        future_version,
                        Some(format!(
                            "Future schema version {} detected; defaults loaded",
                            future_version
                        )),
                    );

                    downgrade_recovered = true;
                    (defaults, false)
                }
            };

        let corruption_detected = settings_corrupt || profiles_corrupt || tags_corrupt;

        // Log migration event (skip if downgrade recovery already logged its own event)
        let is_fresh = !data_dir.join("settings.json").exists() && !settings_corrupt;
        if downgrade_recovered {
            // Already logged DowngradeRecovery above
        } else if corruption_detected {
            log::warn!("Corruption detected in one or more metadata files; defaults loaded");
            migration_logger.log_event(
                MigrationEvent::CorruptionRecovery,
                settings.schema_version,
                Some("One or more metadata files were corrupt; defaults loaded".to_string()),
            );
        } else if is_fresh {
            migration_logger.log_event(MigrationEvent::FreshInstall, settings.schema_version, None);
        } else {
            migration_logger.log_event(
                MigrationEvent::NormalStartup,
                settings.schema_version,
                None,
            );
        }

        // Detect read-only filesystem
        let read_only = Self::detect_read_only(data_dir);
        if read_only {
            log::warn!("Data directory is read-only; metadata writes will be rejected");
        }

        Ok(Self {
            settings_repo,
            profile_repo,
            tag_repo,
            settings: RwLock::new(settings),
            profiles: RwLock::new(profiles),
            custom_tags: RwLock::new(custom_tags),
            built_in_tags,
            read_only: AtomicBool::new(read_only),
            corruption_detected: AtomicBool::new(corruption_detected),
        })
    }

    /// Attempt a test write to detect read-only filesystem.
    fn detect_read_only(data_dir: &Path) -> bool {
        let test_path = data_dir.join(".write_test");
        let result = std::fs::write(&test_path, b"test");
        if result.is_ok() {
            let _ = std::fs::remove_file(&test_path);
            false
        } else {
            true
        }
    }

    fn guard_writable(&self) -> Result<(), IpcError> {
        if self.read_only.load(Ordering::Relaxed) {
            return Err(IpcError {
                code: "STORE_READ_ONLY".to_string(),
                message: "Metadata store is read-only".to_string(),
                detail: Some(
                    "The data directory is not writable. Changes cannot be saved.".to_string(),
                ),
                subsystem: "persistence".to_string(),
            });
        }
        Ok(())
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only.load(Ordering::Relaxed)
    }

    pub fn was_corruption_detected(&self) -> bool {
        self.corruption_detected.load(Ordering::Relaxed)
    }

    // ──────────── Settings ────────────

    pub fn get_settings(&self) -> Result<SettingsGetResponse, IpcError> {
        let s = self.settings.read().map_err(|_| lock_poisoned())?;
        Ok(SettingsGetResponse {
            schema_version: s.schema_version,
            theme_preference: s.theme_preference.clone(),
            tray_enabled: s.tray_enabled,
            launch_at_login: s.launch_at_login,
            privacy_blackout_enabled: s.privacy_blackout_enabled,
            selected_model_id: s.selected_model_id.clone(),
            last_selected_profile_id: s.last_selected_profile_id.clone(),
            last_successful_model_id: s.last_successful_model_id.clone(),
            visual_style: s.visual_style.clone(),
            motion_preference: s.motion_preference.clone(),
            model_aliases: s.model_aliases.clone(),
        })
    }

    pub fn update_settings(&self, req: &SettingsUpdateRequest) -> Result<(), IpcError> {
        self.guard_writable()?;

        let mut s = self.settings.write().map_err(|_| lock_poisoned())?;

        if let Some(ref tp) = req.theme_preference {
            s.theme_preference = tp.clone();
        }
        if let Some(te) = req.tray_enabled {
            s.tray_enabled = te;
        }
        if let Some(lal) = req.launch_at_login {
            s.launch_at_login = lal;
        }
        if let Some(pbe) = req.privacy_blackout_enabled {
            s.privacy_blackout_enabled = pbe;
        }
        if let Some(ref smi) = req.selected_model_id {
            s.selected_model_id = smi.clone();
        }
        if let Some(ref lspi) = req.last_selected_profile_id {
            s.last_selected_profile_id = lspi.clone();
        }
        if let Some(ref lsmi) = req.last_successful_model_id {
            s.last_successful_model_id = lsmi.clone();
        }
        if let Some(ref vs) = req.visual_style {
            s.visual_style = vs.clone();
        }
        if let Some(ref mp) = req.motion_preference {
            s.motion_preference = mp.clone();
        }

        self.settings_repo.save(&s)?;
        Ok(())
    }

    // ──────────── Model Aliases ────────────

    pub fn set_model_alias(&self, model_id: &str, alias: &str) -> Result<(), IpcError> {
        self.guard_writable()?;
        let mut s = self.settings.write().map_err(|_| lock_poisoned())?;
        s.model_aliases
            .insert(model_id.to_string(), alias.to_string());
        self.settings_repo.save(&s)?;
        Ok(())
    }

    pub fn clear_model_alias(&self, model_id: &str) -> Result<(), IpcError> {
        self.guard_writable()?;
        let mut s = self.settings.write().map_err(|_| lock_poisoned())?;
        s.model_aliases.remove(model_id);
        self.settings_repo.save(&s)?;
        Ok(())
    }

    // ──────────── Profiles ────────────

    pub fn list_profiles(&self) -> Result<Vec<ProfileEntry>, IpcError> {
        let profiles = self.profiles.read().map_err(|_| lock_poisoned())?;
        Ok(profiles.iter().map(profile_to_entry).collect())
    }

    pub fn create_profile(
        &self,
        req: &ProfileCreateRequest,
    ) -> Result<ProfileCreateResponse, IpcError> {
        self.guard_writable()?;
        ProfileRepository::validate_name(&req.name)?;
        ProfileRepository::validate_instruction_body(&req.instruction_body)?;

        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();

        let profile = PromptProfile {
            id: id.clone(),
            name: req.name.trim().to_string(),
            instruction_body: req.instruction_body.clone(),
            is_factory_default: false,
            created_at: now.clone(),
            updated_at: now,
        };

        let mut profiles = self.profiles.write().map_err(|_| lock_poisoned())?;
        profiles.push(profile);
        self.profile_repo.save(&profiles)?;

        Ok(ProfileCreateResponse { id })
    }

    pub fn update_profile(&self, req: &ProfileUpdateRequest) -> Result<(), IpcError> {
        self.guard_writable()?;

        if let Some(ref name) = req.name {
            ProfileRepository::validate_name(name)?;
        }
        if let Some(ref body) = req.instruction_body {
            ProfileRepository::validate_instruction_body(body)?;
        }

        let mut profiles = self.profiles.write().map_err(|_| lock_poisoned())?;
        let profile = profiles
            .iter_mut()
            .find(|p| p.id == req.id)
            .ok_or_else(|| profile_not_found(&req.id))?;

        if profile.is_factory_default {
            return Err(IpcError {
                code: "FACTORY_DEFAULT_IMMUTABLE".to_string(),
                message: "The factory default profile cannot be modified".to_string(),
                detail: None,
                subsystem: "persistence".to_string(),
            });
        }

        if let Some(ref name) = req.name {
            profile.name = name.trim().to_string();
        }
        if let Some(ref body) = req.instruction_body {
            profile.instruction_body = body.clone();
        }
        profile.updated_at = chrono::Utc::now().to_rfc3339();

        self.profile_repo.save(&profiles)?;
        Ok(())
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), IpcError> {
        self.guard_writable()?;
        ProfileRepository::guard_not_factory_default(id)?;

        let mut profiles = self.profiles.write().map_err(|_| lock_poisoned())?;
        let idx = profiles
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| profile_not_found(id))?;
        profiles.remove(idx);
        self.profile_repo.save(&profiles)?;
        Ok(())
    }

    pub fn reset_profile_to_default(&self, id: &str) -> Result<(), IpcError> {
        self.guard_writable()?;
        ProfileRepository::guard_is_factory_default(id)?;

        let mut profiles = self.profiles.write().map_err(|_| lock_poisoned())?;
        let profile = profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| profile_not_found(id))?;

        profile.instruction_body = ProfileRepository::default_instruction_body().to_string();
        profile.updated_at = chrono::Utc::now().to_rfc3339();

        self.profile_repo.save(&profiles)?;
        Ok(())
    }

    /// Get a profile by ID. Returns a clone.
    pub fn get_profile_by_id(&self, id: &str) -> Result<PromptProfile, IpcError> {
        let profiles = self.profiles.read().map_err(|_| lock_poisoned())?;
        profiles
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or_else(|| profile_not_found(id))
    }

    // ──────────── Tags ────────────

    pub fn list_tags(&self) -> Result<(Vec<BuiltInTagEntry>, Vec<CustomTagEntry>), IpcError> {
        let custom = self.custom_tags.read().map_err(|_| lock_poisoned())?;

        let built_in_entries: Vec<BuiltInTagEntry> = self
            .built_in_tags
            .iter()
            .map(builtin_tag_to_entry)
            .collect();
        let custom_entries: Vec<CustomTagEntry> = custom.iter().map(custom_tag_to_entry).collect();

        Ok((built_in_entries, custom_entries))
    }

    pub fn create_tag(&self, req: &TagCreateRequest) -> Result<TagCreateResponse, IpcError> {
        self.guard_writable()?;
        TagRepository::validate_name(&req.name)?;
        TagRepository::validate_instruction_body(&req.instruction_body)?;

        let mut custom = self.custom_tags.write().map_err(|_| lock_poisoned())?;
        TagRepository::guard_no_duplicate_name(&req.name, &custom, None)?;

        let now = chrono::Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();

        let tag = CustomTag {
            id: id.clone(),
            name: req.name.trim().to_string(),
            category: req.category.clone(),
            instruction_body: req.instruction_body.clone(),
            is_built_in: false,
            created_at: now.clone(),
            updated_at: now,
        };

        custom.push(tag);
        self.tag_repo.save_custom_tags(&custom)?;

        Ok(TagCreateResponse { id })
    }

    pub fn update_tag(&self, req: &TagUpdateRequest) -> Result<(), IpcError> {
        self.guard_writable()?;
        TagRepository::guard_not_built_in(&req.id)?;

        if let Some(ref name) = req.name {
            TagRepository::validate_name(name)?;
        }
        if let Some(ref body) = req.instruction_body {
            TagRepository::validate_instruction_body(body)?;
        }

        let mut custom = self.custom_tags.write().map_err(|_| lock_poisoned())?;

        if let Some(ref name) = req.name {
            TagRepository::guard_no_duplicate_name(name, &custom, Some(&req.id))?;
        }

        let tag = custom
            .iter_mut()
            .find(|t| t.id == req.id)
            .ok_or_else(|| tag_not_found(&req.id))?;

        if let Some(ref name) = req.name {
            tag.name = name.trim().to_string();
        }
        if let Some(ref category) = req.category {
            tag.category = category.clone();
        }
        if let Some(ref body) = req.instruction_body {
            tag.instruction_body = body.clone();
        }
        tag.updated_at = chrono::Utc::now().to_rfc3339();

        self.tag_repo.save_custom_tags(&custom)?;
        Ok(())
    }

    pub fn delete_tag(&self, id: &str) -> Result<(), IpcError> {
        self.guard_writable()?;
        TagRepository::guard_not_built_in(id)?;

        let mut custom = self.custom_tags.write().map_err(|_| lock_poisoned())?;
        let idx = custom
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| tag_not_found(id))?;
        custom.remove(idx);
        self.tag_repo.save_custom_tags(&custom)?;
        Ok(())
    }
}

// ──────────── Helpers ────────────

fn lock_poisoned() -> IpcError {
    IpcError {
        code: "INTERNAL_ERROR".to_string(),
        message: "Internal lock poisoned".to_string(),
        detail: None,
        subsystem: "persistence".to_string(),
    }
}

fn profile_not_found(id: &str) -> IpcError {
    IpcError {
        code: "PROFILE_NOT_FOUND".to_string(),
        message: format!("Profile not found: {}", id),
        detail: None,
        subsystem: "persistence".to_string(),
    }
}

fn tag_not_found(id: &str) -> IpcError {
    IpcError {
        code: "TAG_NOT_FOUND".to_string(),
        message: format!("Tag not found: {}", id),
        detail: None,
        subsystem: "persistence".to_string(),
    }
}

fn profile_to_entry(p: &PromptProfile) -> ProfileEntry {
    ProfileEntry {
        id: p.id.clone(),
        name: p.name.clone(),
        instruction_body: if p.is_factory_default {
            String::new()
        } else {
            p.instruction_body.clone()
        },
        is_factory_default: p.is_factory_default,
        created_at: p.created_at.clone(),
        updated_at: p.updated_at.clone(),
    }
}

fn builtin_tag_to_entry(t: &BuiltInTag) -> BuiltInTagEntry {
    BuiltInTagEntry {
        id: t.id.clone(),
        name: t.name.clone(),
        category: t.category.clone(),
        instruction_body: t.instruction_body.clone(),
        is_built_in: t.is_built_in,
        balancing_group: t.balancing_group.clone(),
    }
}

fn custom_tag_to_entry(t: &CustomTag) -> CustomTagEntry {
    CustomTagEntry {
        id: t.id.clone(),
        name: t.name.clone(),
        category: t.category.clone(),
        instruction_body: t.instruction_body.clone(),
        is_built_in: t.is_built_in,
        created_at: t.created_at.clone(),
        updated_at: t.updated_at.clone(),
    }
}
