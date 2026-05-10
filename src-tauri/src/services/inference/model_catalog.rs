// Phase: 9
// Model catalog — discovery and registry for local model files.
//
// Discovers models from:
// 1. Bundled models dir: {tauri_resource_dir}/models/ (or MODUTONE_BUNDLED_MODELS_DIR env var)
// 2. User models dir: {app_data_dir}/models/ (or MODUTONE_USER_MODELS_DIR env var)
//
// Each directory may contain a model_catalog.json describing available models.
// Discovery checks whether the actual GGUF file or MLX model directory exists
// on disk and is supported by the current platform.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Inference backend needed to load a discovered model.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelBackend {
    #[default]
    Gguf,
    Mlx,
}

/// A single entry in model_catalog.json.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogEntry {
    pub model_id: String,
    pub display_name: String,
    #[serde(default)]
    pub backend: ModelBackend,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    pub size_bytes: u64,
    pub min_ram_bytes: u64,
    pub ram_class_label: String,
}

impl CatalogEntry {
    fn storage_path(&self) -> Result<&str, String> {
        match self.backend {
            ModelBackend::Gguf => self
                .filename
                .as_deref()
                .or(self.path.as_deref())
                .ok_or_else(|| "GGUF catalog entries require filename".to_string()),
            ModelBackend::Mlx => self
                .path
                .as_deref()
                .or(self.filename.as_deref())
                .ok_or_else(|| "MLX catalog entries require path".to_string()),
        }
    }
}

/// A model discovered at runtime with resolved path and install status.
#[derive(Debug, Clone)]
pub struct DiscoveredModel {
    pub id: String,
    pub display_name: String,
    pub filename: String,
    pub backend: ModelBackend,
    pub size_bytes: u64,
    pub min_ram_bytes: u64,
    pub ram_class_label: String,
    pub model_path: PathBuf,
    pub is_installed: bool,
    pub is_cataloged: bool,
    pub quant_label: Option<String>,
}

/// Registry of discovered models. Created during app setup, managed as Tauri state.
pub struct ModelRegistry {
    models: Vec<DiscoveredModel>,
}

impl ModelRegistry {
    /// Initialize the model registry by discovering models from known directories.
    ///
    /// - `app_data_dir`: The app's data directory (for user models)
    pub fn init(app_data_dir: &Path, resource_dir: Option<&Path>) -> Self {
        let mut models = Vec::new();

        // 1. Bundled models directory
        let bundled_dir = resolve_bundled_models_dir(resource_dir);
        if let Some(dir) = &bundled_dir {
            log::info!("Scanning bundled models dir: {}", dir.display());
            if let Ok(entries) = discover_from_directory(dir) {
                models.extend(entries);
            }
        }

        // 2. User models directory (auto-create if missing)
        let user_dir = resolve_user_models_dir(app_data_dir);
        if let Err(e) = std::fs::create_dir_all(&user_dir) {
            log::warn!(
                "Failed to create user models directory {}: {}",
                user_dir.display(),
                e
            );
        }
        log::info!("Scanning user models dir: {}", user_dir.display());
        if let Ok(entries) = discover_from_directory(&user_dir) {
            // User entries override bundled entries with the same modelId
            for entry in entries {
                if let Some(existing) = models.iter_mut().find(|m| m.id == entry.id) {
                    *existing = entry;
                } else {
                    models.push(entry);
                }
            }
        }

        log::info!(
            "Model registry initialized: {} model(s), {} installed",
            models.len(),
            models.iter().filter(|m| m.is_installed).count()
        );

        Self { models }
    }

    /// Get all discovered models.
    pub fn models(&self) -> &[DiscoveredModel] {
        &self.models
    }

    /// Find a model by ID.
    pub fn find_by_id(&self, model_id: &str) -> Option<&DiscoveredModel> {
        self.models.iter().find(|m| m.id == model_id)
    }
}

/// Resolve bundled models directory.
/// Priority: MODUTONE_BUNDLED_MODELS_DIR env var, then Tauri's resource
/// directory. Tauri owns the platform-specific bundle layout, including:
///
///   Windows NSIS: resource dir beside the executable
///   macOS .app:   Contents/Resources
///   Linux DEB:    /usr/lib/{package-name}
///   AppImage:     ${APPDIR}/usr/lib/{package-name}
fn resolve_bundled_models_dir(resource_dir: Option<&Path>) -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("MODUTONE_BUNDLED_MODELS_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return Some(path);
        }
        log::warn!(
            "MODUTONE_BUNDLED_MODELS_DIR set but not a directory: {}",
            path.display()
        );
    }

    // In debug/dev builds, prefer the source tree resources directory.
    // This ensures the developer's actual local models in src-tauri/resources/models/
    // are found, even when target/debug/models/ exists (Tauri copies resources there).
    #[cfg(debug_assertions)]
    {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let dev_models = PathBuf::from(manifest_dir).join("resources").join("models");
        if dev_models.is_dir() {
            log::info!(
                "Dev mode: using source tree models dir: {}",
                dev_models.display()
            );
            return Some(dev_models);
        }
    }

    if let Some(dir) = resource_dir.and_then(resolve_resource_models_dir) {
        log::info!("Tauri resource dir: using models dir: {}", dir.display());
        return Some(dir);
    }

    if let Some(resource_dir) = resource_dir {
        let models_dir = resource_dir.join("models");
        log::warn!(
            "Tauri resource dir resolved but models dir is missing: {}",
            models_dir.display()
        );
    }

    None
}

fn resolve_resource_models_dir(resource_dir: &Path) -> Option<PathBuf> {
    let models_dir = resource_dir.join("models");
    if models_dir.is_dir() {
        Some(models_dir)
    } else {
        None
    }
}

/// Resolve user models directory.
/// Priority: MODUTONE_USER_MODELS_DIR env var, then {app_data_dir}/models/
fn resolve_user_models_dir(app_data_dir: &Path) -> PathBuf {
    if let Ok(dir) = std::env::var("MODUTONE_USER_MODELS_DIR") {
        return PathBuf::from(dir);
    }
    app_data_dir.join("models")
}

/// Parse a model_catalog.json file into catalog entries.
pub fn parse_catalog(json: &str) -> Result<Vec<CatalogEntry>, String> {
    serde_json::from_str(json).map_err(|e| format!("Invalid model catalog JSON: {}", e))
}

/// Estimate minimum RAM from file size (×2 for weights + KV cache + overhead).
fn estimate_min_ram_bytes(file_size: u64) -> u64 {
    file_size.saturating_mul(2)
}

/// Compute a human-readable RAM class label from estimated min RAM bytes.
/// Includes " est." suffix to indicate it's a heuristic, not curated data.
fn estimate_ram_class_label(min_ram_bytes: u64) -> String {
    let gb = min_ram_bytes as f64 / 1_000_000_000.0;
    let bucket: u64 = if gb <= 4.0 {
        4
    } else if gb <= 8.0 {
        8
    } else if gb <= 16.0 {
        16
    } else if gb <= 32.0 {
        32
    } else if gb <= 64.0 {
        64
    } else {
        (gb / 8.0).ceil() as u64 * 8
    };
    format!("~{} GB est.", bucket)
}

/// Parsed shard info from a GGUF filename like `model-00001-of-00003.gguf`.
#[derive(Debug)]
struct ShardInfo {
    base_name: String,
    shard_index: u32,
    total_shards: u32,
}

/// Try to parse shard info from a GGUF filename.
/// Returns `None` if the filename is not a shard (i.e., it's a single-file model).
///
/// Expected pattern: `{base}-{NNNNN}-of-{TTTTT}.gguf`
fn parse_shard_filename(filename: &str) -> Option<ShardInfo> {
    let stem = filename
        .strip_suffix(".gguf")
        .or_else(|| filename.strip_suffix(".GGUF"))?;

    // Find the last "-of-" to split off the total shards count
    let of_pos = stem.rfind("-of-")?;
    let total_str = &stem[of_pos + 4..];
    let total: u32 = total_str.parse().ok()?;

    // The part before "-of-" should end with "-{NNNNN}"
    let before_of = &stem[..of_pos];
    let dash_pos = before_of.rfind('-')?;
    let index_str = &before_of[dash_pos + 1..];
    let index: u32 = index_str.parse().ok()?;

    let base_name = before_of[..dash_pos].to_string();

    if index == 0 || total == 0 || index > total || base_name.is_empty() {
        return None;
    }

    Some(ShardInfo {
        base_name,
        shard_index: index,
        total_shards: total,
    })
}

/// Minimum ratio of actual file size to expected catalog size for a file
/// to be considered validly installed. Below this threshold the file is
/// treated as incomplete (e.g. interrupted download, truncated merge).
const INSTALLED_SIZE_THRESHOLD: f64 = 0.9;

/// Validate that a model file is installed: exists AND has acceptable size.
/// If `expected_size` is 0, only existence is checked.
fn validate_file_installed(path: &Path, expected_size: u64) -> bool {
    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if !metadata.is_file() {
        return false;
    }
    if expected_size == 0 {
        return true;
    }
    let actual_size = metadata.len();
    let min_acceptable = (expected_size as f64 * INSTALLED_SIZE_THRESHOLD) as u64;
    if actual_size < min_acceptable {
        log::warn!(
            "Model file size mismatch: expected {} bytes, actual {} bytes (threshold {}). \
             File may be incomplete.",
            expected_size,
            actual_size,
            min_acceptable
        );
        return false;
    }
    true
}

fn backend_supported(backend: ModelBackend) -> bool {
    match backend {
        ModelBackend::Gguf => true,
        ModelBackend::Mlx => cfg!(all(target_os = "macos", target_arch = "aarch64")),
    }
}

fn validate_mlx_model_installed(path: &Path) -> bool {
    if !backend_supported(ModelBackend::Mlx) {
        return false;
    }
    looks_like_mlx_model_dir(path)
}

fn looks_like_mlx_model_dir(path: &Path) -> bool {
    path.is_dir()
        && path.join("config.json").is_file()
        && path.join("tokenizer.json").is_file()
        && directory_has_extension(path, "safetensors")
}

fn directory_has_extension(path: &Path, extension: &str) -> bool {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return false,
    };

    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
        {
            return true;
        }
    }

    false
}

fn directory_size_bytes(path: &Path) -> u64 {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return 0,
    };

    let mut total = 0;
    for entry in entries.filter_map(|entry| entry.ok()) {
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if metadata.is_file() {
            total += metadata.len();
        } else if metadata.is_dir() {
            total += directory_size_bytes(&path);
        }
    }

    total
}

/// Check if a string starts with a quantization prefix (case-insensitive).
/// Matches: q[0-9], iq[0-9], f[0-9], bf[0-9]
fn starts_with_quant_prefix(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() {
        return false;
    }
    match b[0] {
        b'q' => b.len() > 1 && b[1].is_ascii_digit(),
        b'i' => b.len() > 2 && b[1] == b'q' && b[2].is_ascii_digit(),
        b'f' => b.len() > 1 && b[1].is_ascii_digit(),
        b'b' => b.len() > 2 && b[1] == b'f' && b[2].is_ascii_digit(),
        _ => false,
    }
}

/// Parse a GGUF filename stem into a clean display name and optional quant label.
///
/// Examples:
/// - `"Mistral-7B-Instruct-v0.3-Q4_K_M"` → `("Mistral 7B Instruct V0.3", Some("Q4_K_M"))`
/// - `"llama-3.1-8b-instruct-q5_k_m"` → `("Llama 3.1 8B Instruct", Some("Q5_K_M"))`
/// - `"phi-3-mini-4k-instruct"` → `("Phi 3 Mini 4K Instruct", None)`
fn parse_gguf_stem(stem: &str) -> (String, Option<String>) {
    let lower = stem.to_ascii_lowercase();
    let bytes = lower.as_bytes();

    // Find the position of the last separator before a quant-like suffix
    let mut quant_sep_pos: Option<usize> = None;
    for i in (0..bytes.len()).rev() {
        if bytes[i] == b'-' || bytes[i] == b'_' || bytes[i] == b'.' {
            let after = &lower[i + 1..];
            if starts_with_quant_prefix(after) {
                quant_sep_pos = Some(i);
                break;
            }
        }
    }

    let (name_part, quant_label) = if let Some(sep_pos) = quant_sep_pos {
        let quant = stem[sep_pos + 1..].to_uppercase();
        let name = &stem[..sep_pos];
        (name.to_string(), Some(quant))
    } else {
        (stem.to_string(), None)
    };

    // Strip trailing -gguf / _gguf (case-insensitive)
    let name_lower = name_part.to_ascii_lowercase();
    let name_part = if name_lower.ends_with("-gguf") || name_lower.ends_with("_gguf") {
        name_part[..name_part.len() - 5].to_string()
    } else {
        name_part
    };

    // Replace `-` and `_` with spaces, title-case each word, collapse spaces
    let clean: String = name_part
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            // Short words with mixed digits+letters → uppercase all letters
            // (handles size tokens: 8b→8B, 4k→4K, 14b→14B, v0.3→V0.3)
            let has_digit = word.bytes().any(|b| b.is_ascii_digit());
            let has_alpha = word.bytes().any(|b| b.is_ascii_alphabetic());
            if has_digit && has_alpha && word.len() <= 4 {
                return word.to_uppercase();
            }
            // Standard title case: capitalize first letter only
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    (clean, quant_label)
}

fn parse_model_dir_name(dirname: &str) -> String {
    dirname
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            if word.eq_ignore_ascii_case("mlx") {
                return "MLX".to_string();
            }
            if word.eq_ignore_ascii_case("gpt") {
                return "GPT".to_string();
            }
            if word.eq_ignore_ascii_case("oss") {
                return "OSS".to_string();
            }
            if word.eq_ignore_ascii_case("tq3") {
                return "TQ3".to_string();
            }
            let has_digit = word.bytes().any(|b| b.is_ascii_digit());
            let has_alpha = word.bytes().any(|b| b.is_ascii_alphabetic());
            if has_digit && has_alpha && word.len() <= 4 {
                return word.to_uppercase();
            }
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Discover models from a directory.
///
/// Phase 1: Read model_catalog.json for curated catalog entries.
/// Phase 2: Scan for *.gguf files not already matched by catalog entries.
fn discover_from_directory(dir: &Path) -> Result<Vec<DiscoveredModel>, String> {
    let mut models = Vec::new();
    let mut cataloged_filenames: HashSet<String> = HashSet::new();
    let mut cataloged_stems: HashSet<String> = HashSet::new();
    let mut cataloged_model_paths: HashSet<String> = HashSet::new();

    // Phase 1: Catalog-driven discovery
    let catalog_path = dir.join("model_catalog.json");
    if catalog_path.exists() {
        let json = std::fs::read_to_string(&catalog_path)
            .map_err(|e| format!("Failed to read {}: {}", catalog_path.display(), e))?;

        let entries = parse_catalog(&json)?;

        for entry in entries {
            let storage_path = entry
                .storage_path()
                .map_err(|e| format!("Invalid catalog entry '{}': {}", entry.model_id, e))?
                .to_string();
            let model_path = dir.join(&storage_path);

            let is_installed = match entry.backend {
                ModelBackend::Gguf => {
                    // Track both the full filename and the stem (without .gguf).
                    // The stem is used to deduplicate shard groups whose base_name
                    // matches a cataloged merged file.
                    if let Some(stem) = storage_path
                        .strip_suffix(".gguf")
                        .or_else(|| storage_path.strip_suffix(".GGUF"))
                    {
                        cataloged_stems.insert(stem.to_string());
                    }
                    cataloged_filenames.insert(storage_path.clone());

                    // Validate both existence and file size. A file that exists
                    // but is significantly smaller than expected is not installed.
                    let installed = validate_file_installed(&model_path, entry.size_bytes);

                    // Only suppress shard groups if the merged file is valid.
                    if !installed {
                        if let Some(stem) = storage_path
                            .strip_suffix(".gguf")
                            .or_else(|| storage_path.strip_suffix(".GGUF"))
                        {
                            cataloged_stems.remove(stem);
                        }
                    }

                    installed
                }
                ModelBackend::Mlx => {
                    cataloged_model_paths.insert(storage_path.clone());
                    validate_mlx_model_installed(&model_path)
                }
            };

            models.push(DiscoveredModel {
                id: entry.model_id,
                display_name: entry.display_name,
                filename: storage_path,
                backend: entry.backend,
                size_bytes: entry.size_bytes,
                min_ram_bytes: entry.min_ram_bytes,
                ram_class_label: entry.ram_class_label,
                model_path,
                is_installed,
                is_cataloged: true,
                quant_label: None,
            });
        }
    }

    // Phase 2: Uncataloged GGUF file discovery (with shard support)
    if dir.is_dir() {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

        // Temporary struct for collecting GGUF file info
        struct GgufFile {
            filename: String,
            path: PathBuf,
            size: u64,
        }

        let mut single_files: Vec<GgufFile> = Vec::new();
        let mut shard_groups: HashMap<String, Vec<(ShardInfo, GgufFile)>> = HashMap::new();

        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let is_gguf = path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"));
            if !is_gguf || !path.is_file() {
                continue;
            }

            let filename = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            if cataloged_filenames.contains(&filename) {
                continue;
            }

            let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if file_size == 0 {
                continue;
            }

            let gguf = GgufFile {
                filename: filename.clone(),
                path,
                size: file_size,
            };

            if let Some(shard_info) = parse_shard_filename(&filename) {
                log::debug!(
                    "Found shard file: {} (part {}/{})",
                    filename,
                    shard_info.shard_index,
                    shard_info.total_shards
                );
                shard_groups
                    .entry(shard_info.base_name.clone())
                    .or_default()
                    .push((shard_info, gguf));
            } else {
                log::debug!("Found single GGUF file: {} ({} bytes)", filename, file_size);
                single_files.push(gguf);
            }
        }

        // Process single (non-sharded) files
        for file in single_files {
            let stem = Path::new(&file.filename)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| file.filename.clone());

            let min_ram = estimate_min_ram_bytes(file.size);
            let ram_label = estimate_ram_class_label(min_ram);

            let (clean_name, quant_label) = parse_gguf_stem(&stem);

            models.push(DiscoveredModel {
                id: format!("local-{}-{}", stem, file.size),
                display_name: clean_name,
                filename: file.filename,
                backend: ModelBackend::Gguf,
                size_bytes: file.size,
                min_ram_bytes: min_ram,
                ram_class_label: ram_label,
                model_path: file.path,
                is_installed: true,
                is_cataloged: false,
                quant_label,
            });
        }

        // Process shard groups — each group becomes one logical model
        for (base_name, mut shards) in shard_groups {
            // Skip shard groups whose base_name matches a cataloged filename
            // stem. This prevents duplicate entries when a merged single-file
            // GGUF coexists with its original shard files in the same directory.
            if cataloged_stems.contains(&base_name) {
                log::debug!(
                    "Skipping shard group '{}': matches cataloged model stem",
                    base_name
                );
                continue;
            }

            shards.sort_by_key(|(info, _)| info.shard_index);

            let total_expected = shards
                .first()
                .map(|(info, _)| info.total_shards)
                .unwrap_or(0);
            let total_size: u64 = shards.iter().map(|(_, f)| f.size).sum();
            let is_complete = shards.len() as u32 == total_expected;

            log::info!(
                "Shard group '{}': {}/{} shards present, total {} bytes, complete={}",
                base_name,
                shards.len(),
                total_expected,
                total_size,
                is_complete
            );

            // Use first shard as the entry point (llama.cpp finds remaining shards automatically)
            let first_shard = &shards[0].1;

            let min_ram = estimate_min_ram_bytes(total_size);
            let ram_label = estimate_ram_class_label(min_ram);

            let (clean_name, quant_label) = parse_gguf_stem(&base_name);

            models.push(DiscoveredModel {
                id: format!("local-{}-{}", base_name, total_size),
                display_name: clean_name,
                filename: first_shard.filename.clone(),
                backend: ModelBackend::Gguf,
                size_bytes: total_size,
                min_ram_bytes: min_ram,
                ram_class_label: ram_label,
                model_path: first_shard.path.clone(),
                is_installed: is_complete,
                is_cataloged: false,
                quant_label,
            });
        }

        for entry in std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?
            .filter_map(|entry| entry.ok())
        {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let dirname = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            if cataloged_model_paths.contains(&dirname) || !looks_like_mlx_model_dir(&path) {
                continue;
            }

            let size = directory_size_bytes(&path);
            let min_ram = estimate_min_ram_bytes(size);
            let ram_label = estimate_ram_class_label(min_ram);
            let display_name = parse_model_dir_name(&dirname);

            models.push(DiscoveredModel {
                id: format!("local-mlx-{}-{}", dirname, size),
                display_name,
                filename: dirname,
                backend: ModelBackend::Mlx,
                size_bytes: size,
                min_ram_bytes: min_ram,
                ram_class_label: ram_label,
                model_path: path,
                is_installed: backend_supported(ModelBackend::Mlx),
                is_cataloged: false,
                quant_label: Some("MLX".to_string()),
            });
        }
    }

    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const VALID_CATALOG: &str = r#"[
        {
            "modelId": "qwen2.5-3b-instruct",
            "displayName": "Qwen 2.5 3B Instruct",
            "filename": "qwen2.5-3b-instruct-q5_k_m.gguf",
            "sizeBytes": 2438740384,
            "minRamBytes": 8000000000,
            "ramClassLabel": "~8 GB"
        },
        {
            "modelId": "qwen2.5-14b-instruct",
            "displayName": "Qwen 2.5 14B Instruct",
            "filename": "qwen2.5-14b-instruct-q5_k_m.gguf",
            "sizeBytes": 2742898688,
            "minRamBytes": 24000000000,
            "ramClassLabel": "~24 GB"
        }
    ]"#;

    #[test]
    fn parse_catalog_from_json() {
        let entries = parse_catalog(VALID_CATALOG).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].model_id, "qwen2.5-3b-instruct");
        assert_eq!(entries[0].display_name, "Qwen 2.5 3B Instruct");
        assert_eq!(
            entries[0].filename.as_deref(),
            Some("qwen2.5-3b-instruct-q5_k_m.gguf")
        );
        assert_eq!(entries[0].backend, ModelBackend::Gguf);
        assert_eq!(entries[0].size_bytes, 2_438_740_384);
        assert_eq!(entries[0].min_ram_bytes, 8_000_000_000);
        assert_eq!(entries[0].ram_class_label, "~8 GB");
        assert_eq!(entries[1].model_id, "qwen2.5-14b-instruct");
    }

    #[test]
    fn parse_catalog_supports_mlx_entry() {
        let json = r#"[
            {
                "modelId": "gpt-oss-20b-tq3",
                "displayName": "GPT-OSS 20B TurboQuant 3-bit",
                "backend": "mlx",
                "path": "gpt-oss-20b-tq3",
                "sizeBytes": 9970000000,
                "minRamBytes": 16000000000,
                "ramClassLabel": "~16 GB"
            }
        ]"#;

        let entries = parse_catalog(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].backend, ModelBackend::Mlx);
        assert_eq!(entries[0].path.as_deref(), Some("gpt-oss-20b-tq3"));
        assert_eq!(entries[0].storage_path().unwrap(), "gpt-oss-20b-tq3");
    }

    #[test]
    fn parse_catalog_empty_file() {
        let entries = parse_catalog("[]").unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn parse_catalog_invalid_json() {
        let result = parse_catalog("not json");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid model catalog JSON"));
    }

    #[test]
    fn parse_catalog_missing_field() {
        let json = r#"[{"modelId": "test"}]"#;
        let result = parse_catalog(json);
        assert!(result.is_err());
    }

    #[test]
    fn discovery_missing_directory() {
        let result = discover_from_directory(Path::new("/nonexistent/path"));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn discovery_catalog_with_missing_gguf() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("model_catalog.json"), VALID_CATALOG).unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 2);
        assert!(!models[0].is_installed);
        assert!(!models[1].is_installed);
        assert_eq!(models[0].id, "qwen2.5-3b-instruct");
        assert!(models[0].is_cataloged);
        assert!(models[1].is_cataloged);
    }

    #[test]
    fn discovery_catalog_with_existing_gguf() {
        let tmp = tempfile::tempdir().unwrap();
        // Use sizeBytes matching our stub file so size validation passes
        let catalog = r#"[
            {
                "modelId": "qwen2.5-3b-instruct",
                "displayName": "Qwen 2.5 3B Instruct",
                "filename": "qwen2.5-3b-instruct-q5_k_m.gguf",
                "sizeBytes": 5,
                "minRamBytes": 8000000000,
                "ramClassLabel": "~8 GB"
            },
            {
                "modelId": "qwen2.5-14b-instruct",
                "displayName": "Qwen 2.5 14B Instruct",
                "filename": "qwen2.5-14b-instruct-q5_k_m.gguf",
                "sizeBytes": 2742898688,
                "minRamBytes": 24000000000,
                "ramClassLabel": "~24 GB"
            }
        ]"#;
        fs::write(tmp.path().join("model_catalog.json"), catalog).unwrap();
        // Create a dummy GGUF file for the first model (5 bytes matches sizeBytes above)
        fs::write(tmp.path().join("qwen2.5-3b-instruct-q5_k_m.gguf"), "dummy").unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 2);
        assert!(models[0].is_installed);
        assert!(models[0].is_cataloged);
        assert!(!models[1].is_installed);
        assert!(models[1].is_cataloged);
    }

    #[test]
    fn discovery_catalog_with_mlx_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let catalog = r#"[
            {
                "modelId": "gpt-oss-20b-tq3",
                "displayName": "GPT-OSS 20B TurboQuant 3-bit",
                "backend": "mlx",
                "path": "gpt-oss-20b-tq3",
                "sizeBytes": 100,
                "minRamBytes": 16000000000,
                "ramClassLabel": "~16 GB"
            }
        ]"#;
        fs::write(tmp.path().join("model_catalog.json"), catalog).unwrap();
        let model_dir = tmp.path().join("gpt-oss-20b-tq3");
        fs::create_dir(&model_dir).unwrap();
        fs::write(model_dir.join("config.json"), "{}").unwrap();
        fs::write(model_dir.join("tokenizer.json"), "{}").unwrap();
        fs::write(model_dir.join("model.safetensors"), "weights").unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].backend, ModelBackend::Mlx);
        assert_eq!(models[0].filename, "gpt-oss-20b-tq3");
        assert_eq!(models[0].is_installed, backend_supported(ModelBackend::Mlx));
    }

    #[test]
    fn discovery_empty_catalog_file() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("model_catalog.json"), "[]").unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn discovery_uncataloged_gguf_detected() {
        let tmp = tempfile::tempdir().unwrap();
        // No catalog, just a raw GGUF file
        fs::write(tmp.path().join("my-custom-model.gguf"), "gguf-content-here").unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 1);
        assert!(models[0].is_installed);
        assert!(!models[0].is_cataloged);
        assert!(models[0].id.starts_with("local-my-custom-model-"));
        assert_eq!(models[0].display_name, "My Custom Model");
        assert_eq!(models[0].filename, "my-custom-model.gguf");
        assert!(models[0].ram_class_label.contains("est."));
        assert!(models[0].size_bytes > 0);
    }

    #[test]
    fn discovery_uncataloged_alongside_catalog() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("model_catalog.json"), VALID_CATALOG).unwrap();
        // Catalog model file
        fs::write(tmp.path().join("qwen2.5-3b-instruct-q5_k_m.gguf"), "dummy").unwrap();
        // Extra uncataloged GGUF
        fs::write(tmp.path().join("uncataloged-model.gguf"), "extra-gguf-data").unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 3); // 2 catalog + 1 uncataloged

        let cataloged: Vec<_> = models.iter().filter(|m| m.is_cataloged).collect();
        let uncataloged: Vec<_> = models.iter().filter(|m| !m.is_cataloged).collect();
        assert_eq!(cataloged.len(), 2);
        assert_eq!(uncataloged.len(), 1);
        assert_eq!(uncataloged[0].display_name, "Uncataloged Model");
        assert!(uncataloged[0].is_installed);
    }

    #[test]
    fn discovery_uncataloged_skips_catalog_matched() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("model_catalog.json"), VALID_CATALOG).unwrap();
        // File matches a catalog entry — should NOT create a duplicate local entry
        fs::write(tmp.path().join("qwen2.5-3b-instruct-q5_k_m.gguf"), "dummy").unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        // Should be exactly 2 (both catalog entries), not 3
        assert_eq!(models.len(), 2);
        assert!(models.iter().all(|m| m.is_cataloged));
    }

    #[test]
    fn discovery_uncataloged_zero_byte_files_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        // Zero-byte file should be skipped
        fs::write(tmp.path().join("empty.gguf"), "").unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert!(models.is_empty());
    }

    #[test]
    fn discovery_uncataloged_id_includes_size_for_collision_resistance() {
        let tmp = tempfile::tempdir().unwrap();
        let content = "gguf-data-payload";
        fs::write(tmp.path().join("test-model.gguf"), content).unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 1);
        let expected_id = format!("local-test-model-{}", content.len());
        assert_eq!(models[0].id, expected_id);
    }

    #[test]
    fn estimate_ram_class_label_values() {
        // 1 GB file → 2 GB min → ~4 GB est.
        assert_eq!(estimate_ram_class_label(2_000_000_000), "~4 GB est.");
        // 2 GB file → 4 GB min → ~4 GB est.
        assert_eq!(estimate_ram_class_label(4_000_000_000), "~4 GB est.");
        // 3 GB file → 6 GB min → ~8 GB est.
        assert_eq!(estimate_ram_class_label(6_000_000_000), "~8 GB est.");
        // 5 GB file → 10 GB min → ~16 GB est.
        assert_eq!(estimate_ram_class_label(10_000_000_000), "~16 GB est.");
        // 10 GB file → 20 GB min → ~32 GB est.
        assert_eq!(estimate_ram_class_label(20_000_000_000), "~32 GB est.");
        // 20 GB file → 40 GB min → ~64 GB est.
        assert_eq!(estimate_ram_class_label(40_000_000_000), "~64 GB est.");
    }

    #[test]
    fn registry_find_by_id() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("model_catalog.json"), VALID_CATALOG).unwrap();

        // Set env var to use temp dir as bundled models dir
        let key = "MODUTONE_BUNDLED_MODELS_DIR";
        let prev = std::env::var(key).ok();
        std::env::set_var(key, tmp.path());

        let registry = ModelRegistry::init(Path::new("/nonexistent"), None);

        // Restore env var
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }

        assert!(registry.find_by_id("qwen2.5-3b-instruct").is_some());
        assert!(registry.find_by_id("nonexistent").is_none());
    }

    #[test]
    fn registry_user_overrides_bundled() {
        let bundled_dir = tempfile::tempdir().unwrap();
        let user_dir = tempfile::tempdir().unwrap();
        let user_models_dir = user_dir.path().join("models");
        fs::create_dir_all(&user_models_dir).unwrap();

        // Bundled has both models, neither installed
        fs::write(bundled_dir.path().join("model_catalog.json"), VALID_CATALOG).unwrap();

        // User dir has the 3B model with a GGUF file present
        // sizeBytes=5 matches the "dummy" stub file (5 bytes)
        let user_catalog = r#"[{
            "modelId": "qwen2.5-3b-instruct",
            "displayName": "Qwen 2.5 3B (User)",
            "filename": "qwen2.5-3b-instruct-q5_k_m.gguf",
            "sizeBytes": 5,
            "minRamBytes": 8000000000,
            "ramClassLabel": "~8 GB"
        }]"#;
        fs::write(user_models_dir.join("model_catalog.json"), user_catalog).unwrap();
        fs::write(
            user_models_dir.join("qwen2.5-3b-instruct-q5_k_m.gguf"),
            "dummy",
        )
        .unwrap();

        // Set env vars
        let bkey = "MODUTONE_BUNDLED_MODELS_DIR";
        let ukey = "MODUTONE_USER_MODELS_DIR";
        let prev_b = std::env::var(bkey).ok();
        let prev_u = std::env::var(ukey).ok();
        std::env::set_var(bkey, bundled_dir.path());
        std::env::set_var(ukey, &user_models_dir);

        let registry = ModelRegistry::init(user_dir.path(), None);

        // Restore
        match prev_b {
            Some(v) => std::env::set_var(bkey, v),
            None => std::env::remove_var(bkey),
        }
        match prev_u {
            Some(v) => std::env::set_var(ukey, v),
            None => std::env::remove_var(ukey),
        }

        // The 3B model should be overridden by user entry (installed, user display name)
        let model_3b = registry.find_by_id("qwen2.5-3b-instruct").unwrap();
        assert!(model_3b.is_installed);
        assert_eq!(model_3b.display_name, "Qwen 2.5 3B (User)");

        // The 14B model should still be from bundled
        let model_14b = registry.find_by_id("qwen2.5-14b-instruct").unwrap();
        assert!(!model_14b.is_installed);
    }

    #[test]
    fn bundled_models_dir_resolves_from_tauri_resource_dir() {
        let resource_dir = tempfile::tempdir().unwrap();
        let models_dir = resource_dir.path().join("models");
        fs::create_dir_all(&models_dir).unwrap();

        let resolved = resolve_resource_models_dir(resource_dir.path());

        assert_eq!(resolved.as_deref(), Some(models_dir.as_path()));
    }

    // --- Shard parsing tests ---

    #[test]
    fn parse_shard_valid_3_of_3() {
        let info = parse_shard_filename("model-q5_k_m-00001-of-00003.gguf").unwrap();
        assert_eq!(info.base_name, "model-q5_k_m");
        assert_eq!(info.shard_index, 1);
        assert_eq!(info.total_shards, 3);
    }

    #[test]
    fn parse_shard_middle_shard() {
        let info = parse_shard_filename("model-q5_k_m-00002-of-00003.gguf").unwrap();
        assert_eq!(info.base_name, "model-q5_k_m");
        assert_eq!(info.shard_index, 2);
        assert_eq!(info.total_shards, 3);
    }

    #[test]
    fn parse_shard_single_file_not_shard() {
        // Single-file GGUFs should NOT parse as shards
        assert!(parse_shard_filename("qwen2.5-3b-instruct-q5_k_m.gguf").is_none());
    }

    #[test]
    fn parse_shard_invalid_index_zero() {
        assert!(parse_shard_filename("model-00000-of-00003.gguf").is_none());
    }

    #[test]
    fn parse_shard_index_exceeds_total() {
        assert!(parse_shard_filename("model-00004-of-00003.gguf").is_none());
    }

    #[test]
    fn parse_shard_no_gguf_extension() {
        assert!(parse_shard_filename("model-00001-of-00003.bin").is_none());
    }

    #[test]
    fn parse_shard_empty_base_name() {
        // "-00001-of-00003.gguf" has empty base
        assert!(parse_shard_filename("-00001-of-00003.gguf").is_none());
    }

    // --- Shard discovery tests ---

    #[test]
    fn discovery_sharded_model_grouped() {
        let tmp = tempfile::tempdir().unwrap();
        // Create 3 shard files for a 14B model
        fs::write(
            tmp.path().join("qwen2.5-14b-q5_k_m-00001-of-00003.gguf"),
            "a".repeat(100),
        )
        .unwrap();
        fs::write(
            tmp.path().join("qwen2.5-14b-q5_k_m-00002-of-00003.gguf"),
            "b".repeat(80),
        )
        .unwrap();
        fs::write(
            tmp.path().join("qwen2.5-14b-q5_k_m-00003-of-00003.gguf"),
            "c".repeat(60),
        )
        .unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 1); // grouped into one logical model

        let model = &models[0];
        assert_eq!(model.display_name, "Qwen2.5 14B");
        assert_eq!(model.quant_label, Some("Q5_K_M".to_string()));
        assert_eq!(model.size_bytes, 240); // 100 + 80 + 60
        assert!(model.is_installed); // all 3 of 3 present
        assert!(!model.is_cataloged);
        assert!(model.id.starts_with("local-qwen2.5-14b-q5_k_m-"));
        // Entry point should be the first shard
        assert!(model
            .filename
            .contains("qwen2.5-14b-q5_k_m-00001-of-00003.gguf"));
        assert!(model.ram_class_label.contains("est."));
    }

    #[test]
    fn discovery_sharded_incomplete_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        // Only 2 of 3 shards present
        fs::write(tmp.path().join("model-00001-of-00003.gguf"), "a".repeat(50)).unwrap();
        fs::write(tmp.path().join("model-00003-of-00003.gguf"), "c".repeat(50)).unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 1);
        assert!(!models[0].is_installed); // incomplete shard set
        assert_eq!(models[0].display_name, "Model");
        assert_eq!(models[0].size_bytes, 100); // sum of present shards
    }

    #[test]
    fn discovery_shards_deduped_when_catalog_has_merged_file() {
        let tmp = tempfile::tempdir().unwrap();
        // Catalog has a merged single-file entry.
        // sizeBytes=11 matches the "merged-data" stub (11 bytes) so it passes
        // size validation and its stem suppresses the shard group.
        let catalog = r#"[{
            "modelId": "qwen2.5-14b-instruct",
            "displayName": "Qwen 2.5 14B Instruct",
            "filename": "qwen2.5-14b-instruct-q5_k_m.gguf",
            "sizeBytes": 11,
            "minRamBytes": 24000000000,
            "ramClassLabel": "~24 GB"
        }]"#;
        fs::write(tmp.path().join("model_catalog.json"), catalog).unwrap();
        // Merged file exists (installed) — 11 bytes matches sizeBytes above
        fs::write(
            tmp.path().join("qwen2.5-14b-instruct-q5_k_m.gguf"),
            "merged-data",
        )
        .unwrap();
        // Shard files also present (leftover from before merge)
        fs::write(
            tmp.path()
                .join("qwen2.5-14b-instruct-q5_k_m-00001-of-00003.gguf"),
            "a".repeat(100),
        )
        .unwrap();
        fs::write(
            tmp.path()
                .join("qwen2.5-14b-instruct-q5_k_m-00002-of-00003.gguf"),
            "b".repeat(100),
        )
        .unwrap();
        fs::write(
            tmp.path()
                .join("qwen2.5-14b-instruct-q5_k_m-00003-of-00003.gguf"),
            "c".repeat(100),
        )
        .unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        // Should be exactly 1 (the cataloged entry), NOT 2
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "qwen2.5-14b-instruct");
        assert!(models[0].is_cataloged);
        assert!(models[0].is_installed);
    }

    #[test]
    fn discovery_sharded_alongside_single() {
        let tmp = tempfile::tempdir().unwrap();
        // One single-file model
        fs::write(tmp.path().join("small-model.gguf"), "small-data").unwrap();
        // One sharded model (2 parts)
        fs::write(
            tmp.path().join("big-model-00001-of-00002.gguf"),
            "a".repeat(200),
        )
        .unwrap();
        fs::write(
            tmp.path().join("big-model-00002-of-00002.gguf"),
            "b".repeat(150),
        )
        .unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 2); // 1 single + 1 sharded group

        let single: Vec<_> = models
            .iter()
            .filter(|m| m.display_name == "Small Model")
            .collect();
        let sharded: Vec<_> = models
            .iter()
            .filter(|m| m.display_name == "Big Model")
            .collect();
        assert_eq!(single.len(), 1);
        assert_eq!(sharded.len(), 1);
        assert!(single[0].is_installed);
        assert!(sharded[0].is_installed);
        assert_eq!(sharded[0].size_bytes, 350);
    }

    // --- File size validation tests ---

    #[test]
    fn validate_file_installed_missing_file() {
        assert!(!validate_file_installed(
            Path::new("/nonexistent/file.gguf"),
            1000
        ));
    }

    #[test]
    fn validate_file_installed_zero_expected_bypasses_check() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tiny.gguf");
        fs::write(&path, "x").unwrap();
        // expected_size=0 means "no size info, just check existence"
        assert!(validate_file_installed(&path, 0));
    }

    #[test]
    fn validate_file_installed_exact_size() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("model.gguf");
        let data = "a".repeat(1000);
        fs::write(&path, &data).unwrap();
        assert!(validate_file_installed(&path, 1000));
    }

    #[test]
    fn validate_file_installed_slightly_smaller_passes() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("model.gguf");
        // 95% of expected — above the 90% threshold
        let data = "a".repeat(950);
        fs::write(&path, &data).unwrap();
        assert!(validate_file_installed(&path, 1000));
    }

    #[test]
    fn validate_file_installed_too_small_fails() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("model.gguf");
        // 50% of expected — well below 90% threshold (truncated)
        let data = "a".repeat(500);
        fs::write(&path, &data).unwrap();
        assert!(!validate_file_installed(&path, 1000));
    }

    #[test]
    fn validate_file_installed_at_threshold_boundary() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("model.gguf");
        // Exactly 90% of expected — at the boundary (should pass)
        let data = "a".repeat(900);
        fs::write(&path, &data).unwrap();
        assert!(validate_file_installed(&path, 1000));
    }

    #[test]
    fn validate_file_installed_just_below_threshold() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("model.gguf");
        // 89.9% — just below threshold (should fail)
        let data = "a".repeat(899);
        fs::write(&path, &data).unwrap();
        assert!(!validate_file_installed(&path, 1000));
    }

    #[test]
    fn discovery_catalog_truncated_file_not_installed() {
        let tmp = tempfile::tempdir().unwrap();
        let catalog = r#"[{
            "modelId": "big-model",
            "displayName": "Big Model",
            "filename": "big-model.gguf",
            "sizeBytes": 10000,
            "minRamBytes": 8000000000,
            "ramClassLabel": "~8 GB"
        }]"#;
        fs::write(tmp.path().join("model_catalog.json"), catalog).unwrap();
        // File exists but is way too small (truncated download)
        fs::write(tmp.path().join("big-model.gguf"), "a".repeat(100)).unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        assert_eq!(models.len(), 1);
        assert!(!models[0].is_installed); // truncated → not installed
        assert!(models[0].is_cataloged);
    }

    #[test]
    fn discovery_shards_surface_when_catalog_merged_file_truncated() {
        let tmp = tempfile::tempdir().unwrap();
        // Catalog points to a merged file, but that file is truncated
        let catalog = r#"[{
            "modelId": "big-model",
            "displayName": "Big Model",
            "filename": "big-model-q5_k_m.gguf",
            "sizeBytes": 10000,
            "minRamBytes": 24000000000,
            "ramClassLabel": "~24 GB"
        }]"#;
        fs::write(tmp.path().join("model_catalog.json"), catalog).unwrap();
        // Truncated merged file
        fs::write(tmp.path().join("big-model-q5_k_m.gguf"), "x".repeat(100)).unwrap();
        // Valid shard files present
        fs::write(
            tmp.path().join("big-model-q5_k_m-00001-of-00002.gguf"),
            "a".repeat(5000),
        )
        .unwrap();
        fs::write(
            tmp.path().join("big-model-q5_k_m-00002-of-00002.gguf"),
            "b".repeat(5000),
        )
        .unwrap();

        let models = discover_from_directory(tmp.path()).unwrap();
        // Should be 2: the catalog entry (not installed) + shard group (installed)
        assert_eq!(models.len(), 2);

        let catalog_model = models.iter().find(|m| m.is_cataloged).unwrap();
        assert!(!catalog_model.is_installed);

        let shard_model = models.iter().find(|m| !m.is_cataloged).unwrap();
        assert!(shard_model.is_installed);
        assert_eq!(shard_model.size_bytes, 10000);
    }

    // --- parse_gguf_stem tests ---

    #[test]
    fn parse_stem_with_quant_q4_k_m() {
        let (name, quant) = parse_gguf_stem("Mistral-7B-Instruct-v0.3-Q4_K_M");
        assert_eq!(name, "Mistral 7B Instruct V0.3");
        assert_eq!(quant, Some("Q4_K_M".to_string()));
    }

    #[test]
    fn parse_stem_with_lowercase_quant() {
        let (name, quant) = parse_gguf_stem("llama-3.1-8b-instruct-q5_k_m");
        assert_eq!(name, "Llama 3.1 8B Instruct");
        assert_eq!(quant, Some("Q5_K_M".to_string()));
    }

    #[test]
    fn parse_stem_without_quant() {
        let (name, quant) = parse_gguf_stem("phi-3-mini-4k-instruct");
        assert_eq!(name, "Phi 3 Mini 4K Instruct");
        assert_eq!(quant, None);
    }

    #[test]
    fn parse_stem_q4_0_format() {
        let (name, quant) = parse_gguf_stem("phi-3-mini-4k-instruct-q4_0");
        assert_eq!(name, "Phi 3 Mini 4K Instruct");
        assert_eq!(quant, Some("Q4_0".to_string()));
    }

    #[test]
    fn parse_stem_f16_format() {
        let (name, quant) = parse_gguf_stem("model-name-f16");
        assert_eq!(name, "Model Name");
        assert_eq!(quant, Some("F16".to_string()));
    }

    #[test]
    fn parse_stem_bf16_format() {
        let (name, quant) = parse_gguf_stem("model-name-bf16");
        assert_eq!(name, "Model Name");
        assert_eq!(quant, Some("BF16".to_string()));
    }

    #[test]
    fn parse_stem_iq_format() {
        let (name, quant) = parse_gguf_stem("model-iq2_xxs");
        assert_eq!(name, "Model");
        assert_eq!(quant, Some("IQ2_XXS".to_string()));
    }

    #[test]
    fn parse_stem_strips_gguf_suffix() {
        let (name, quant) = parse_gguf_stem("my-model-gguf-q4_k_m");
        assert_eq!(name, "My Model");
        assert_eq!(quant, Some("Q4_K_M".to_string()));

        let (name2, quant2) = parse_gguf_stem("my-model-gguf");
        assert_eq!(name2, "My Model");
        assert_eq!(quant2, None);
    }

    #[test]
    fn parse_stem_plain_name() {
        let (name, quant) = parse_gguf_stem("simple");
        assert_eq!(name, "Simple");
        assert_eq!(quant, None);
    }
}
