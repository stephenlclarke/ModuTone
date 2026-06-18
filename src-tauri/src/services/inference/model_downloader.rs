// In-app model downloader.
//
// Downloads approved catalog models into the user models directory. Downloads
// are explicit user actions and emit progress events; writing uses .partial
// files followed by atomic rename so discovery never sees half-written files.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use reqwest::header::{CONTENT_LENGTH, RANGE};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;

use crate::contracts::events::{ModelDownloadProgressEvent, ModelDownloadStatus};
use crate::services::inference::model_catalog::{CatalogEntry, ModelBackend, ModelRegistry};

#[derive(Clone)]
pub struct DownloadFile {
    pub relative_path: &'static str,
    pub url: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
}

#[derive(Clone)]
pub struct DownloadSpec {
    pub model_id: &'static str,
    pub display_name: &'static str,
    pub backend: ModelBackend,
    pub filename: Option<&'static str>,
    pub path: Option<&'static str>,
    pub files: &'static [DownloadFile],
    pub size_bytes: u64,
    pub min_ram_bytes: u64,
    pub ram_class_label: &'static str,
    pub unsupported_reason: Option<&'static str>,
}

impl DownloadSpec {
    pub fn can_download(&self) -> bool {
        self.unsupported_reason.is_none()
    }

    pub fn catalog_entry(&self) -> CatalogEntry {
        CatalogEntry {
            model_id: self.model_id.to_string(),
            display_name: self.display_name.to_string(),
            backend: self.backend,
            filename: self.filename.map(ToString::to_string),
            path: self.path.map(ToString::to_string),
            files: self
                .files
                .iter()
                .map(|file| file.relative_path.to_string())
                .collect(),
            size_bytes: self.size_bytes,
            min_ram_bytes: self.min_ram_bytes,
            ram_class_label: self.ram_class_label.to_string(),
        }
    }
}

const QWEN_3B_FILES: &[DownloadFile] = &[DownloadFile {
    relative_path: "qwen2.5-3b-instruct-q5_k_m.gguf",
    url: "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q5_k_m.gguf",
    size_bytes: 2_438_740_384,
    sha256: "2c63dde5f2c9ab1fd64d47dee2d34dade6ba9ff62442d1d20b5342310c982081",
}];

const QWEN_14B_FILES: &[DownloadFile] = &[
    DownloadFile {
        relative_path: "qwen2.5-14b-instruct-q5_k_m-00001-of-00003.gguf",
        url: "https://huggingface.co/Qwen/Qwen2.5-14B-Instruct-GGUF/resolve/main/qwen2.5-14b-instruct-q5_k_m-00001-of-00003.gguf",
        size_bytes: 4_005_690_208,
        sha256: "5899521e6d7196db09bc78dc50c066b1514e48f0cd6ae085e481ee06e2134ac6",
    },
    DownloadFile {
        relative_path: "qwen2.5-14b-instruct-q5_k_m-00002-of-00003.gguf",
        url: "https://huggingface.co/Qwen/Qwen2.5-14B-Instruct-GGUF/resolve/main/qwen2.5-14b-instruct-q5_k_m-00002-of-00003.gguf",
        size_bytes: 3_997_407_296,
        sha256: "ada99d5a4222c0d2b0394f35ad4cd4281bdd1d8b7e9fa4be4f80f036621c580a",
    },
    DownloadFile {
        relative_path: "qwen2.5-14b-instruct-q5_k_m-00003-of-00003.gguf",
        url: "https://huggingface.co/Qwen/Qwen2.5-14B-Instruct-GGUF/resolve/main/qwen2.5-14b-instruct-q5_k_m-00003-of-00003.gguf",
        size_bytes: 2_505_775_872,
        sha256: "02a06964fa37f5e9cd21c92aa3851309ae3f3dcc2f52d631f1ce31b5b93640c0",
    },
];

const GPT_OSS_FILES: &[DownloadFile] = &[
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/.gitattributes",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/.gitattributes",
        size_bytes: 1_570,
        sha256: "34448b82c17d60fec9b65b1f093c115ddbaadc04beb1b0140b6bfed2e012a930",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/README.md",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/README.md",
        size_bytes: 4_626,
        sha256: "6e21f20a453200e94fc9dfd8ea7736fb95a90902a4f44e77e346fc177fa53693",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/chat_template.jinja",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/chat_template.jinja",
        size_bytes: 16_738,
        sha256: "a4c9919cbbd4acdd51ccffe22da049264b1b73e59055fa58811a99efbd7c8146",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/config.json",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/config.json",
        size_bytes: 2_742,
        sha256: "c937fd01d002ecb557ccda1e5bb15b103f4f882597f20a61b60a894c5242d5f8",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/generation_config.json",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/generation_config.json",
        size_bytes: 177,
        sha256: "f9970ada892d2d1f72e3ed0a6535ccebadd11897318794ca671d8c7014c957da",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/model-00001-of-00002.safetensors",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/model-00001-of-00002.safetensors",
        size_bytes: 5_311_240_112,
        sha256: "2c945029397b0102696875713794c8a9e8c4a19a8e4b523343328e29e3ca23e0",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/model-00002-of-00002.safetensors",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/model-00002-of-00002.safetensors",
        size_bytes: 4_632_424_562,
        sha256: "cfe802b748f23a713d16f35d74cc07ac818a2c20f8d8c641ebcee0d4cf808cbe",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/model.safetensors.index.json",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/model.safetensors.index.json",
        size_bytes: 84_666,
        sha256: "953cc52d6378824b130f62b0fd3362365587461a940b5e39998db93520f1d218",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/model_card.md",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/model_card.md",
        size_bytes: 2_181,
        sha256: "ee406ef67660a192d5e40dbbd27d3c93ad169502407b3879cc4225491c437649",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/tokenizer.json",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/tokenizer.json",
        size_bytes: 27_868_174,
        sha256: "0614fe83cadab421296e664e1f48f4261fa8fef6e03e63bb75c20f38e37d07d3",
    },
    DownloadFile {
        relative_path: "gpt-oss-20b-tq3/tokenizer_config.json",
        url: "https://huggingface.co/manjunathshiva/gpt-oss-20b-tq3/resolve/main/tokenizer_config.json",
        size_bytes: 351,
        sha256: "e8bd3a5467e803377983a664138bf28bc3c9bf5c8134b57202d6ec22fa627cbf",
    },
];

pub fn download_spec_for_model(model_id: &str) -> Option<DownloadSpec> {
    match model_id {
        "qwen2.5-3b-instruct" => Some(DownloadSpec {
            model_id: "qwen2.5-3b-instruct",
            display_name: "Qwen 2.5 3B Instruct",
            backend: ModelBackend::Gguf,
            filename: Some("qwen2.5-3b-instruct-q5_k_m.gguf"),
            path: None,
            files: QWEN_3B_FILES,
            size_bytes: 2_438_740_384,
            min_ram_bytes: 8_000_000_000,
            ram_class_label: "~8 GB",
            unsupported_reason: None,
        }),
        "qwen2.5-14b-instruct" => Some(DownloadSpec {
            model_id: "qwen2.5-14b-instruct",
            display_name: "Qwen 2.5 14B Instruct",
            backend: ModelBackend::Gguf,
            filename: Some("qwen2.5-14b-instruct-q5_k_m-00001-of-00003.gguf"),
            path: None,
            files: QWEN_14B_FILES,
            size_bytes: 10_508_873_376,
            min_ram_bytes: 24_000_000_000,
            ram_class_label: "~24 GB",
            unsupported_reason: None,
        }),
        "gpt-oss-20b-tq3" => Some(DownloadSpec {
            model_id: "gpt-oss-20b-tq3",
            display_name: "GPT-OSS 20B TurboQuant 3-bit",
            backend: ModelBackend::Mlx,
            filename: None,
            path: Some("gpt-oss-20b-tq3"),
            files: GPT_OSS_FILES,
            size_bytes: 9_971_645_899,
            min_ram_bytes: 16_000_000_000,
            ram_class_label: "~16 GB",
            unsupported_reason: gpt_oss_download_unsupported_reason(),
        }),
        _ => None,
    }
}

fn gpt_oss_download_unsupported_reason() -> Option<&'static str> {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        None
    } else {
        Some("GPT-OSS 20B TQ3 requires Apple Silicon macOS and the MLX runtime")
    }
}

#[derive(Clone, Default)]
pub struct ModelDownloadManager {
    active: Arc<Mutex<HashMap<String, Arc<AtomicBool>>>>,
}

impl ModelDownloadManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn start(
        &self,
        app: AppHandle,
        registry: Arc<std::sync::Mutex<ModelRegistry>>,
        model_id: String,
    ) -> Result<DownloadStart, String> {
        let spec = download_spec_for_model(&model_id)
            .ok_or_else(|| format!("No download source is configured for model '{}'", model_id))?;

        if let Some(reason) = spec.unsupported_reason {
            return Err(reason.to_string());
        }

        if registry
            .lock()
            .map_err(|e| format!("Model registry lock failed: {}", e))?
            .find_by_id(&model_id)
            .is_some_and(|model| model.is_installed)
        {
            return Ok(DownloadStart {
                started: false,
                already_installed: true,
                total_bytes: spec.size_bytes,
            });
        }

        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active.lock().await;
            if active.contains_key(&model_id) {
                return Ok(DownloadStart {
                    started: false,
                    already_installed: false,
                    total_bytes: spec.size_bytes,
                });
            }
            active.insert(model_id.clone(), cancel.clone());
        }

        let manager = self.clone();
        let total_bytes = spec.size_bytes;
        let spawned_spec = spec.clone();
        tauri::async_runtime::spawn(async move {
            emit_progress(
                &app,
                &model_id,
                ModelDownloadStatus::Queued,
                0,
                spawned_spec.size_bytes,
                None,
                None,
            );

            let result =
                download_model_files(&app, &model_id, &spawned_spec, &registry, cancel.clone())
                    .await;
            manager.active.lock().await.remove(&model_id);

            match result {
                Ok(()) => {
                    emit_progress(
                        &app,
                        &model_id,
                        ModelDownloadStatus::Completed,
                        spawned_spec.size_bytes,
                        spawned_spec.size_bytes,
                        None,
                        None,
                    );
                }
                Err(DownloadFailure::Canceled(downloaded)) => {
                    emit_progress(
                        &app,
                        &model_id,
                        ModelDownloadStatus::Canceled,
                        downloaded,
                        spawned_spec.size_bytes,
                        None,
                        None,
                    );
                }
                Err(DownloadFailure::Failed { downloaded, error }) => {
                    emit_progress(
                        &app,
                        &model_id,
                        ModelDownloadStatus::Failed,
                        downloaded,
                        spawned_spec.size_bytes,
                        None,
                        Some(error),
                    );
                }
            }
        });

        Ok(DownloadStart {
            started: true,
            already_installed: false,
            total_bytes,
        })
    }

    pub async fn cancel(&self, model_id: &str) -> bool {
        let active = self.active.lock().await;
        if let Some(cancel) = active.get(model_id) {
            cancel.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }
}

pub struct DownloadStart {
    pub started: bool,
    pub already_installed: bool,
    pub total_bytes: u64,
}

enum DownloadFailure {
    Canceled(u64),
    Failed { downloaded: u64, error: String },
}

async fn download_model_files(
    app: &AppHandle,
    model_id: &str,
    spec: &DownloadSpec,
    registry: &Arc<std::sync::Mutex<ModelRegistry>>,
    cancel: Arc<AtomicBool>,
) -> Result<(), DownloadFailure> {
    let user_models_dir = registry
        .lock()
        .map_err(|e| DownloadFailure::Failed {
            downloaded: 0,
            error: format!("Model registry lock failed: {}", e),
        })?
        .user_models_dir();

    tokio::fs::create_dir_all(&user_models_dir)
        .await
        .map_err(|e| DownloadFailure::Failed {
            downloaded: 0,
            error: format!("Failed to create models directory: {}", e),
        })?;

    read_user_catalog_entries(&user_models_dir.join("model_catalog.json"))
        .await
        .map_err(|error| DownloadFailure::Failed {
            downloaded: 0,
            error,
        })?;

    let client = reqwest::Client::builder()
        .user_agent("ModuTone/1.0 model downloader")
        .build()
        .map_err(|e| DownloadFailure::Failed {
            downloaded: 0,
            error: format!("Failed to initialize downloader: {}", e),
        })?;

    let mut downloaded = 0;

    for file in spec.files {
        if cancel.load(Ordering::SeqCst) {
            return Err(DownloadFailure::Canceled(downloaded));
        }

        let destination = safe_join(&user_models_dir, file.relative_path)
            .map_err(|error| DownloadFailure::Failed { downloaded, error })?;
        if verify_existing_file(&destination, file)
            .await
            .map_err(|error| DownloadFailure::Failed { downloaded, error })?
        {
            downloaded += file.size_bytes;
            emit_progress(
                app,
                model_id,
                ModelDownloadStatus::Downloading,
                downloaded,
                spec.size_bytes,
                Some(file.relative_path.to_string()),
                None,
            );
            continue;
        }

        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DownloadFailure::Failed {
                    downloaded,
                    error: format!(
                        "Failed to create model directory for {}: {}",
                        file.relative_path, e
                    ),
                })?;
        }

        let file_downloaded = download_file(
            &client,
            app,
            model_id,
            spec,
            file,
            &destination,
            &cancel,
            &mut downloaded,
        )
        .await?;

        downloaded = downloaded.saturating_sub(file_downloaded);
        downloaded += file.size_bytes;
    }

    write_user_catalog_entry(&user_models_dir, spec)
        .await
        .map_err(|error| DownloadFailure::Failed { downloaded, error })?;

    registry
        .lock()
        .map_err(|e| DownloadFailure::Failed {
            downloaded,
            error: format!("Model registry lock failed: {}", e),
        })?
        .refresh();

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn download_file(
    client: &reqwest::Client,
    app: &AppHandle,
    model_id: &str,
    spec: &DownloadSpec,
    file: &DownloadFile,
    destination: &Path,
    cancel: &AtomicBool,
    downloaded: &mut u64,
) -> Result<u64, DownloadFailure> {
    let partial = destination.with_extension(format!(
        "{}partial",
        destination
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| format!("{}.", ext))
            .unwrap_or_default()
    ));
    let mut partial_existing_size = file_size(&partial).unwrap_or(0);
    if partial_existing_size == file.size_bytes {
        match verify_file_checksum(&partial, file).await {
            Ok(()) => {
                tokio::fs::rename(&partial, destination)
                    .await
                    .map_err(|e| DownloadFailure::Failed {
                        downloaded: *downloaded,
                        error: format!(
                            "Failed to install complete partial for {}: {}",
                            file.relative_path, e
                        ),
                    })?;
                *downloaded += file.size_bytes;
                emit_progress(
                    app,
                    model_id,
                    ModelDownloadStatus::Downloading,
                    (*downloaded).min(spec.size_bytes),
                    spec.size_bytes,
                    Some(file.relative_path.to_string()),
                    None,
                );
                return Ok(file.size_bytes);
            }
            Err(error) => {
                log::warn!(
                    "Removing model partial with failed checksum: file={}, error={}",
                    file.relative_path,
                    error
                );
                tokio::fs::remove_file(&partial)
                    .await
                    .map_err(|e| DownloadFailure::Failed {
                        downloaded: *downloaded,
                        error: format!(
                            "Failed to remove invalid partial for {}: {}",
                            file.relative_path, e
                        ),
                    })?;
                partial_existing_size = 0;
            }
        }
    } else if partial_existing_size > file.size_bytes {
        tokio::fs::remove_file(&partial)
            .await
            .map_err(|e| DownloadFailure::Failed {
                downloaded: *downloaded,
                error: format!(
                    "Failed to remove oversized partial for {}: {}",
                    file.relative_path, e
                ),
            })?;
        partial_existing_size = 0;
    }
    let partial_size = if partial_existing_size < file.size_bytes {
        partial_existing_size
    } else {
        0
    };

    let mut request = client.get(file.url);
    if partial_size > 0 {
        request = request.header(RANGE, format!("bytes={}-", partial_size));
    }

    let response = request.send().await.map_err(|e| DownloadFailure::Failed {
        downloaded: *downloaded,
        error: format!("Failed to download {}: {}", file.relative_path, e),
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(DownloadFailure::Failed {
            downloaded: *downloaded,
            error: format!(
                "Download failed for {}: HTTP {}",
                file.relative_path, status
            ),
        });
    }

    let resumes = status == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut output = if resumes {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&partial)
            .await
    } else {
        tokio::fs::File::create(&partial).await
    }
    .map_err(|e| DownloadFailure::Failed {
        downloaded: *downloaded,
        error: format!(
            "Failed to open partial download for {}: {}",
            file.relative_path, e
        ),
    })?;

    if resumes {
        output
            .seek(std::io::SeekFrom::End(0))
            .await
            .map_err(|e| DownloadFailure::Failed {
                downloaded: *downloaded,
                error: format!("Failed to resume {}: {}", file.relative_path, e),
            })?;
    }

    let content_length = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    let mut file_downloaded = if resumes { partial_size } else { 0 };
    if resumes {
        *downloaded += partial_size;
    }

    emit_progress(
        app,
        model_id,
        ModelDownloadStatus::Downloading,
        *downloaded,
        spec.size_bytes,
        Some(file.relative_path.to_string()),
        None,
    );

    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| DownloadFailure::Failed {
            downloaded: *downloaded,
            error: format!("Failed while reading {}: {}", file.relative_path, e),
        })?
    {
        if cancel.load(Ordering::SeqCst) {
            output.flush().await.ok();
            return Err(DownloadFailure::Canceled(*downloaded));
        }

        output
            .write_all(&chunk)
            .await
            .map_err(|e| DownloadFailure::Failed {
                downloaded: *downloaded,
                error: format!("Failed while writing {}: {}", file.relative_path, e),
            })?;
        let chunk_len = chunk.len() as u64;
        file_downloaded += chunk_len;
        *downloaded += chunk_len;

        emit_progress(
            app,
            model_id,
            ModelDownloadStatus::Downloading,
            (*downloaded).min(spec.size_bytes),
            spec.size_bytes,
            Some(file.relative_path.to_string()),
            None,
        );
    }

    output.flush().await.map_err(|e| DownloadFailure::Failed {
        downloaded: *downloaded,
        error: format!("Failed to flush {}: {}", file.relative_path, e),
    })?;
    drop(output);

    if file_downloaded != file.size_bytes {
        return Err(DownloadFailure::Failed {
            downloaded: *downloaded,
            error: format!(
                "Downloaded {} bytes for {}, expected exactly {}{}",
                file_downloaded,
                file.relative_path,
                file.size_bytes,
                content_length
                    .map(|len| format!("; response reported {}", len))
                    .unwrap_or_default()
            ),
        });
    }

    verify_file_checksum(&partial, file)
        .await
        .map_err(|error| DownloadFailure::Failed {
            downloaded: *downloaded,
            error,
        })?;

    tokio::fs::rename(&partial, destination)
        .await
        .map_err(|e| DownloadFailure::Failed {
            downloaded: *downloaded,
            error: format!("Failed to install {}: {}", file.relative_path, e),
        })?;

    Ok(file_downloaded)
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err(format!("Download path must be relative: {}", relative));
    }

    let mut output = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) => output.push(part),
            _ => return Err(format!("Invalid download path: {}", relative)),
        }
    }

    Ok(output)
}

fn file_size(path: &Path) -> Option<u64> {
    std::fs::metadata(path)
        .ok()
        .filter(|metadata| metadata.is_file())
        .map(|metadata| metadata.len())
}

async fn verify_existing_file(path: &Path, file: &DownloadFile) -> Result<bool, String> {
    let Some(size) = file_size(path) else {
        return Ok(false);
    };

    if size != file.size_bytes {
        log::warn!(
            "Removing model file with unexpected size: file={}, actual_bytes={}, expected_bytes={}",
            file.relative_path,
            size,
            file.size_bytes
        );
        tokio::fs::remove_file(path).await.map_err(|e| {
            format!(
                "Failed to remove invalid existing file for {}: {}",
                file.relative_path, e
            )
        })?;
        return Ok(false);
    }

    match verify_file_checksum(path, file).await {
        Ok(()) => Ok(true),
        Err(error) => {
            log::warn!(
                "Removing model file with failed checksum: file={}, error={}",
                file.relative_path,
                error
            );
            tokio::fs::remove_file(path).await.map_err(|e| {
                format!(
                    "Failed to remove invalid existing file for {}: {}",
                    file.relative_path, e
                )
            })?;
            Ok(false)
        }
    }
}

async fn verify_file_checksum(path: &Path, file: &DownloadFile) -> Result<(), String> {
    let actual = sha256_file(path).await?;
    if actual == file.sha256 {
        return Ok(());
    }

    Err(format!(
        "Checksum mismatch for {}: expected {}, got {}",
        file.relative_path, file.sha256, actual
    ))
}

async fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| format!("Failed to open file for checksum: {}", e))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1024 * 1024];

    loop {
        let bytes = file
            .read(&mut buffer)
            .await
            .map_err(|e| format!("Failed to read file for checksum: {}", e))?;
        if bytes == 0 {
            break;
        }
        hasher.update(&buffer[..bytes]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

async fn write_user_catalog_entry(
    user_models_dir: &Path,
    spec: &DownloadSpec,
) -> Result<(), String> {
    let catalog_path = user_models_dir.join("model_catalog.json");
    let mut entries = read_user_catalog_entries(&catalog_path).await?;

    let new_entry = spec.catalog_entry();
    if let Some(existing) = entries
        .iter_mut()
        .find(|entry| entry.model_id == new_entry.model_id)
    {
        *existing = new_entry;
    } else {
        entries.push(new_entry);
    }

    let json = serde_json::to_string_pretty(&entries)
        .map_err(|e| format!("Failed to encode model catalog: {}", e))?;
    let tmp_path = catalog_path.with_extension("json.tmp");
    tokio::fs::write(&tmp_path, format!("{}\n", json))
        .await
        .map_err(|e| format!("Failed to write user model catalog: {}", e))?;
    tokio::fs::rename(&tmp_path, &catalog_path)
        .await
        .map_err(|e| format!("Failed to install user model catalog: {}", e))?;

    Ok(())
}

async fn read_user_catalog_entries(catalog_path: &Path) -> Result<Vec<CatalogEntry>, String> {
    if !catalog_path.is_file() {
        return Ok(Vec::new());
    }

    let json = tokio::fs::read_to_string(catalog_path)
        .await
        .map_err(|e| format!("Failed to read user model catalog: {}", e))?;
    serde_json::from_str::<Vec<CatalogEntry>>(&json)
        .map_err(|e| format!("User model catalog is invalid JSON: {}", e))
}

fn emit_progress(
    app: &AppHandle,
    model_id: &str,
    status: ModelDownloadStatus,
    bytes_downloaded: u64,
    total_bytes: u64,
    file_name: Option<String>,
    error: Option<String>,
) {
    let event = ModelDownloadProgressEvent {
        contract_version: 1,
        model_id: model_id.to_string(),
        status,
        bytes_downloaded,
        total_bytes,
        file_name,
        error,
    };
    if let Err(e) = app.emit("model:download-progress", event) {
        log::warn!("Failed to emit model download progress: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn tiny_download_file(sha256: &'static str) -> DownloadFile {
        DownloadFile {
            relative_path: "tiny-model.gguf",
            url: "https://example.invalid/tiny-model.gguf",
            size_bytes: 5,
            sha256,
        }
    }

    #[test]
    fn qwen_14b_manifest_uses_official_shards() {
        let spec = download_spec_for_model("qwen2.5-14b-instruct").unwrap();
        assert_eq!(spec.files.len(), 3);
        assert_eq!(spec.size_bytes, 10_508_873_376);
        assert!(spec
            .files
            .iter()
            .all(|file| file.relative_path.ends_with(".gguf")));
    }

    #[test]
    fn gpt_oss_manifest_installs_into_model_directory() {
        let spec = download_spec_for_model("gpt-oss-20b-tq3").unwrap();
        assert_eq!(spec.backend, ModelBackend::Mlx);
        assert_eq!(spec.path, Some("gpt-oss-20b-tq3"));
        assert!(spec.files.iter().any(|file| {
            file.relative_path == "gpt-oss-20b-tq3/model-00001-of-00002.safetensors"
        }));
    }

    #[test]
    fn safe_join_rejects_path_traversal() {
        let root = Path::new("/tmp/models");
        assert!(safe_join(root, "../escape.gguf").is_err());
        assert!(safe_join(root, "/tmp/escape.gguf").is_err());
        assert_eq!(
            safe_join(root, "model/file.gguf").unwrap(),
            Path::new("/tmp/models/model/file.gguf")
        );
    }

    #[tokio::test]
    async fn user_catalog_parse_error_does_not_overwrite_catalog() {
        let tmp = tempfile::tempdir().unwrap();
        let catalog_path = tmp.path().join("model_catalog.json");
        tokio::fs::write(&catalog_path, "{invalid json")
            .await
            .unwrap();

        let spec = download_spec_for_model("qwen2.5-3b-instruct").unwrap();
        let err = write_user_catalog_entry(tmp.path(), &spec)
            .await
            .unwrap_err();

        assert!(err.contains("invalid JSON"));
        assert_eq!(
            tokio::fs::read_to_string(&catalog_path).await.unwrap(),
            "{invalid json"
        );
    }

    #[tokio::test]
    async fn verify_existing_file_accepts_exact_size_and_checksum() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tiny-model.gguf");
        tokio::fs::write(&path, b"hello").await.unwrap();

        let file = tiny_download_file(HELLO_SHA256);

        assert!(verify_existing_file(&path, &file).await.unwrap());
        assert!(path.is_file());
    }

    #[tokio::test]
    async fn verify_existing_file_removes_checksum_mismatch() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tiny-model.gguf");
        tokio::fs::write(&path, b"hullo").await.unwrap();

        let file = tiny_download_file(HELLO_SHA256);

        assert!(!verify_existing_file(&path, &file).await.unwrap());
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn verify_existing_file_removes_oversized_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("tiny-model.gguf");
        tokio::fs::write(&path, b"hello!").await.unwrap();

        let file = tiny_download_file(HELLO_SHA256);

        assert!(!verify_existing_file(&path, &file).await.unwrap());
        assert!(!path.exists());
    }
}
