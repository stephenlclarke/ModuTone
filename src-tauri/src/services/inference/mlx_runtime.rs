// Apple Silicon MLX runtime installer.
//
// Creates and maintains a private Python virtual environment in app data so
// installed GUI apps can load MLX models without requiring shell environment
// variables.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Emitter};
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::contracts::events::{MlxRuntimeInstallProgressEvent, MlxRuntimeInstallStatus};

#[derive(Clone)]
pub struct MlxRuntimeManager {
    app_data_dir: PathBuf,
    active: Arc<Mutex<bool>>,
}

impl MlxRuntimeManager {
    pub fn new(app_data_dir: &Path) -> Self {
        Self {
            app_data_dir: app_data_dir.to_path_buf(),
            active: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn status(&self) -> MlxRuntimeStatus {
        let supported = mlx_supported();
        let install_dir = self.install_dir();
        let python_path = self.python_path();
        let installing = *self.active.lock().await;
        let installed = if supported && python_path.is_file() {
            python_is_supported_version(&python_path).await
                && python_has_runtime_packages(&python_path).await
        } else {
            false
        };

        MlxRuntimeStatus {
            supported,
            installed,
            installing,
            install_dir,
            python_path: if installed { Some(python_path) } else { None },
            unavailable_reason: if supported {
                None
            } else {
                Some("MLX runtime setup requires Apple Silicon macOS".to_string())
            },
        }
    }

    pub async fn start_install(&self, app: AppHandle) -> Result<MlxRuntimeInstallStart, String> {
        if !mlx_supported() {
            return Err("MLX runtime setup requires Apple Silicon macOS".to_string());
        }

        let status = self.status().await;
        if status.installed {
            return Ok(MlxRuntimeInstallStart {
                started: false,
                already_installed: true,
                install_dir: status.install_dir,
                python_path: status.python_path,
            });
        }

        {
            let mut active = self.active.lock().await;
            if *active {
                return Ok(MlxRuntimeInstallStart {
                    started: false,
                    already_installed: false,
                    install_dir: self.install_dir(),
                    python_path: Some(self.python_path()),
                });
            }
            *active = true;
        }

        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            emit_progress(
                &app,
                MlxRuntimeInstallStatus::Queued,
                "queued",
                Some("Preparing MLX runtime setup".to_string()),
                None,
            );

            let result = manager.install(&app).await;
            *manager.active.lock().await = false;

            match result {
                Ok(_) => {
                    emit_progress(
                        &app,
                        MlxRuntimeInstallStatus::Completed,
                        "completed",
                        Some("MLX runtime installed".to_string()),
                        None,
                    );
                }
                Err(error) => {
                    emit_progress(
                        &app,
                        MlxRuntimeInstallStatus::Failed,
                        "failed",
                        None,
                        Some(error),
                    );
                }
            }
        });

        Ok(MlxRuntimeInstallStart {
            started: true,
            already_installed: false,
            install_dir: self.install_dir(),
            python_path: Some(self.python_path()),
        })
    }

    fn install_dir(&self) -> PathBuf {
        self.app_data_dir.join("mlx").join(".venv")
    }

    fn python_path(&self) -> PathBuf {
        self.install_dir().join("bin").join("python")
    }

    async fn install(&self, app: &AppHandle) -> Result<PathBuf, String> {
        let bootstrap_python = resolve_bootstrap_python().await?;
        let install_dir = self.install_dir();
        let python_path = self.python_path();

        if let Some(parent) = install_dir.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create MLX runtime directory: {}", e))?;
        }

        emit_progress(
            app,
            MlxRuntimeInstallStatus::Installing,
            "creating_venv",
            Some("Creating Python environment".to_string()),
            None,
        );

        if python_path.is_file() && !python_is_supported_version(&python_path).await {
            tokio::fs::remove_dir_all(&install_dir).await.map_err(|e| {
                format!("Failed to remove unsupported MLX Python environment: {}", e)
            })?;
        }

        if !python_path.is_file() {
            let install_dir_arg = path_arg(&install_dir);
            run_command(
                &bootstrap_python,
                &["-m", "venv", install_dir_arg.as_str()],
                "Failed to create MLX Python environment",
            )
            .await?;
        }

        emit_progress(
            app,
            MlxRuntimeInstallStatus::Installing,
            "upgrading_pip",
            Some("Installing Python packaging tools".to_string()),
            None,
        );
        run_python_module(
            &python_path,
            "pip",
            &["install", "--upgrade", "pip", "setuptools", "wheel"],
            "Failed to install Python packaging tools",
        )
        .await?;

        emit_progress(
            app,
            MlxRuntimeInstallStatus::Installing,
            "installing_packages",
            Some("Installing mlx-lm and TurboQuant MLX packages".to_string()),
            None,
        );
        run_python_module(
            &python_path,
            "pip",
            &[
                "install",
                "huggingface_hub[hf_xet]",
                "mlx-lm>=0.31.3",
                "turboquant-mlx-full>=0.2.0",
            ],
            "Failed to install MLX Python packages",
        )
        .await?;

        emit_progress(
            app,
            MlxRuntimeInstallStatus::Installing,
            "verifying_runtime",
            Some("Verifying MLX runtime imports".to_string()),
            None,
        );
        if !python_has_runtime_packages(&python_path).await {
            return Err("MLX runtime verification failed after installation".to_string());
        }

        Ok(python_path)
    }
}

pub struct MlxRuntimeStatus {
    pub supported: bool,
    pub installed: bool,
    pub installing: bool,
    pub install_dir: PathBuf,
    pub python_path: Option<PathBuf>,
    pub unavailable_reason: Option<String>,
}

pub struct MlxRuntimeInstallStart {
    pub started: bool,
    pub already_installed: bool,
    pub install_dir: PathBuf,
    pub python_path: Option<PathBuf>,
}

fn mlx_supported() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

async fn resolve_bootstrap_python() -> Result<PathBuf, String> {
    for candidate in bootstrap_python_candidates() {
        if candidate.components().count() > 1 && !candidate.exists() {
            continue;
        }

        if python_is_supported_version(&candidate).await {
            return Ok(candidate);
        }
    }

    Err("Python 3.14, 3.13, or 3.12 was not found. Install Python 3.14, then run MLX runtime setup again.".to_string())
}

fn bootstrap_python_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("MODUTONE_MLX_BOOTSTRAP_PYTHON") {
        push_unique(&mut candidates, PathBuf::from(path));
    }
    for version in ["3.14", "3.13", "3.12"] {
        push_unique(
            &mut candidates,
            PathBuf::from(format!("/opt/homebrew/bin/python{}", version)),
        );
        push_unique(
            &mut candidates,
            PathBuf::from(format!("/usr/local/bin/python{}", version)),
        );
        push_unique(&mut candidates, PathBuf::from(format!("python{}", version)));
    }
    push_unique(&mut candidates, PathBuf::from("python3"));
    candidates
}

fn is_supported_python_version(major: u8, minor: u8) -> bool {
    major == 3 && matches!(minor, 12..=14)
}

fn push_unique(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

async fn python_is_supported_version(candidate: &Path) -> bool {
    let output = Command::new(candidate)
        .arg("-c")
        .arg("import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .await
        .ok();

    let Some(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }

    let version = String::from_utf8_lossy(&output.stdout);
    let mut parts = version.trim().split('.');
    let major = parts.next().and_then(|part| part.parse::<u8>().ok());
    let minor = parts.next().and_then(|part| part.parse::<u8>().ok());
    match (major, minor) {
        (Some(major), Some(minor)) => is_supported_python_version(major, minor),
        _ => false,
    }
}

async fn python_has_runtime_packages(python_path: &Path) -> bool {
    let script = "import mlx_lm; import turboquant_mlx.generate";
    Command::new(python_path)
        .arg("-c")
        .arg(script)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|status| status.success())
        .unwrap_or(false)
}

async fn run_python_module(
    python_path: &Path,
    module: &str,
    args: &[&str],
    context: &str,
) -> Result<(), String> {
    let mut command_args = vec!["-m", module];
    command_args.extend_from_slice(args);
    run_command(python_path, &command_args, context).await
}

async fn run_command(program: &Path, args: &[&str], context: &str) -> Result<(), String> {
    let output = Command::new(program)
        .args(args)
        .env("PIP_DISABLE_PIP_VERSION_CHECK", "1")
        .output()
        .await
        .map_err(|e| format!("{}: {}", context, e))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Err(format!(
        "{}: {}{}{}",
        context,
        output.status,
        command_tail("stderr", &stderr),
        command_tail("stdout", &stdout)
    ))
}

fn command_tail(label: &str, text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut lines = trimmed.lines().rev().take(8).collect::<Vec<_>>();
    lines.reverse();
    format!("; {}: {}", label, lines.join(" | "))
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn emit_progress(
    app: &AppHandle,
    status: MlxRuntimeInstallStatus,
    step: &str,
    detail: Option<String>,
    error: Option<String>,
) {
    let event = MlxRuntimeInstallProgressEvent {
        contract_version: 1,
        status,
        step: step.to_string(),
        detail,
        error,
    };
    if let Err(e) = app.emit("mlx:runtime-progress", event) {
        log::warn!("Failed to emit MLX runtime progress: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_candidates_prefer_newer_python_versions() {
        let candidates = bootstrap_python_candidates();
        let py314 = candidates
            .iter()
            .position(|candidate| candidate == &PathBuf::from("/opt/homebrew/bin/python3.14"))
            .expect("missing Python 3.14 candidate");
        let py312 = candidates
            .iter()
            .position(|candidate| candidate == &PathBuf::from("/opt/homebrew/bin/python3.12"))
            .expect("missing Python 3.12 candidate");
        assert!(py314 < py312);
        assert!(candidates.contains(&PathBuf::from("python3.14")));
        assert!(candidates.contains(&PathBuf::from("python3.13")));
        assert!(candidates.contains(&PathBuf::from("python3.12")));
    }

    #[test]
    fn supported_python_versions_cover_tested_bootstrap_range() {
        assert!(is_supported_python_version(3, 14));
        assert!(is_supported_python_version(3, 13));
        assert!(is_supported_python_version(3, 12));
        assert!(!is_supported_python_version(3, 11));
        assert!(!is_supported_python_version(4, 0));
    }

    #[test]
    fn command_tail_includes_recent_lines() {
        let output = command_tail("stderr", "a\nb\nc");
        assert_eq!(output, "; stderr: a | b | c");
    }
}
