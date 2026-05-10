// Phase: 4
// Worker process supervisor — manages the inference worker child process.
//
// Responsibilities:
// - Spawn worker binary as a child process
// - Ready handshake with timeout (15s)
// - Crash detection and restart logic (max 3 restarts in 60s)
// - Graceful + force shutdown (5s timeout)
// - Model loading (warming state)
// - Route worker stdout messages (job messages go to JobCoordinator)
// - Emit runtime:status-changed and worker:crashed events

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::ChildStdin;
use tokio::sync::Mutex;

use crate::contracts::events::{RuntimeStatusChangedEvent, WorkerCrashedEvent};
use crate::contracts::shared::{AppState, WorkerState};

use super::job_coordinator::JobCoordinator;
use super::worker_protocol::{WorkerInbound, WorkerOutbound};

const READY_TIMEOUT: Duration = Duration::from_secs(15);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const RESTART_DELAY: Duration = Duration::from_secs(1);
const MAX_RESTARTS: usize = 3;
const RESTART_WINDOW: Duration = Duration::from_secs(60);
const CONTRACT_VERSION: u32 = 1;

// --- Worker Process State Machine ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerProcessState {
    NotStarted,
    Starting,
    Idle,
    Warming,
    Busy,
    CrashRecovery,
    ShuttingDown,
    Stopped,
}

impl WorkerProcessState {
    pub fn to_worker_state(self) -> WorkerState {
        match self {
            Self::Idle => WorkerState::Idle,
            Self::Warming => WorkerState::Warming,
            Self::Busy => WorkerState::Busy,
            _ => WorkerState::Unavailable,
        }
    }

    pub fn to_app_state(self) -> AppState {
        match self {
            Self::Idle | Self::Warming | Self::Busy => AppState::Ready,
            _ => AppState::Degraded,
        }
    }
}

// --- Supervisor Inner State ---

pub(crate) struct SupervisorInner {
    pub state: WorkerProcessState,
    pub stdin: Option<ChildStdin>,
    pub loaded_model_id: Option<String>,
    pub restart_timestamps: VecDeque<Instant>,
    pub worker_binary_path: PathBuf,
    pub shutdown_requested: bool,
    /// Holds the child process handle to prevent kill_on_drop from
    /// immediately killing the worker when the spawn block ends.
    pub child_process: Option<tokio::process::Child>,
}

// --- Public API ---

#[derive(Clone)]
pub struct WorkerSupervisor {
    pub(crate) inner: Arc<Mutex<SupervisorInner>>,
    /// Channel to request a restart. The restart loop task receives from
    /// this channel and calls spawn_worker, breaking the async type cycle
    /// that would otherwise occur between spawn_worker ↔ schedule_restart.
    restart_tx: tokio::sync::mpsc::Sender<()>,
    restart_rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<()>>>>,
}

impl WorkerSupervisor {
    pub fn new(worker_binary_path: PathBuf) -> Self {
        let (restart_tx, restart_rx) = tokio::sync::mpsc::channel::<()>(4);
        Self {
            inner: Arc::new(Mutex::new(SupervisorInner {
                state: WorkerProcessState::NotStarted,
                stdin: None,
                loaded_model_id: None,
                restart_timestamps: VecDeque::new(),
                worker_binary_path,
                shutdown_requested: false,
                child_process: None,
            })),
            restart_tx,
            restart_rx: Arc::new(Mutex::new(Some(restart_rx))),
        }
    }

    /// Start the worker process and the restart watcher loop.
    /// Called once during app initialization.
    pub async fn start(&self, app: AppHandle, coordinator: JobCoordinator) -> Result<(), String> {
        // Take the restart receiver (only the first call gets it)
        let restart_rx = {
            let mut rx_opt = self.restart_rx.lock().await;
            rx_opt.take()
        };

        if let Some(rx) = restart_rx {
            // Spawn the restart watcher loop
            let supervisor = self.clone();
            let app_clone = app.clone();
            let coordinator_clone = coordinator.clone();
            tokio::spawn(async move {
                restart_watcher_loop(rx, supervisor, app_clone, coordinator_clone).await;
            });
        }

        self.spawn_worker(app, coordinator).await
    }

    /// Get the current worker process state.
    pub async fn get_state(&self) -> WorkerProcessState {
        self.inner.lock().await.state
    }

    /// Get the currently loaded model ID, if any.
    pub async fn get_loaded_model_id(&self) -> Option<String> {
        self.inner.lock().await.loaded_model_id.clone()
    }

    /// Send a message to the worker via stdin.
    pub async fn send_to_worker(&self, msg: &WorkerInbound) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        let stdin = inner
            .stdin
            .as_mut()
            .ok_or_else(|| "Worker stdin not available".to_string())?;

        let json = serde_json::to_string(msg)
            .map_err(|e| format!("Failed to serialize worker message: {}", e))?;

        stdin
            .write_all(json.as_bytes())
            .await
            .map_err(|e| format!("Failed to write to worker stdin: {}", e))?;
        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("Failed to write newline to worker stdin: {}", e))?;
        stdin
            .flush()
            .await
            .map_err(|e| format!("Failed to flush worker stdin: {}", e))?;

        Ok(())
    }

    /// Transition worker state to Busy (called by JobCoordinator on dispatch).
    pub async fn set_busy(&self) {
        let mut inner = self.inner.lock().await;
        if matches!(inner.state, WorkerProcessState::Idle) {
            inner.state = WorkerProcessState::Busy;
        }
    }

    /// Transition worker state to Idle if currently Busy (called when job ends).
    pub async fn transition_idle_if_busy(&self) {
        let mut inner = self.inner.lock().await;
        if inner.state == WorkerProcessState::Busy {
            inner.state = WorkerProcessState::Idle;
        }
    }

    /// Transition worker state to Warming (called by warm_model command).
    pub async fn set_warming(&self) -> Result<(), String> {
        let mut inner = self.inner.lock().await;
        if inner.state != WorkerProcessState::Idle {
            return Err(format!(
                "Cannot warm model: worker state is {:?}",
                inner.state
            ));
        }
        inner.state = WorkerProcessState::Warming;
        Ok(())
    }

    /// Force-kill the worker process. Used when cooperative cancellation
    /// times out and the worker is stuck in a long compute phase.
    /// The stdout reader will detect EOF and trigger crash recovery + restart.
    pub async fn force_kill_worker(&self) {
        let mut inner = self.inner.lock().await;
        if matches!(
            inner.state,
            WorkerProcessState::Busy | WorkerProcessState::Idle | WorkerProcessState::Warming
        ) {
            log::warn!(
                "Force-killing worker process (state={:?}), crash recovery will restart",
                inner.state
            );
            // Drop stdin and child handle — kill_on_drop triggers process kill.
            // State stays as-is; the stdout reader EOF handler will set
            // CrashRecovery and request_restart().
            inner.stdin = None;
            inner.child_process = None;
        }
    }

    /// Graceful shutdown of the worker process.
    pub async fn shutdown(&self, app: &AppHandle) -> Result<(), String> {
        {
            let mut inner = self.inner.lock().await;
            if matches!(
                inner.state,
                WorkerProcessState::Stopped | WorkerProcessState::NotStarted
            ) {
                return Ok(());
            }
            inner.state = WorkerProcessState::ShuttingDown;
            inner.shutdown_requested = true;
        }

        // Send graceful shutdown message
        let _ = self.send_to_worker(&WorkerInbound::Shutdown).await;

        // Wait for process exit with timeout
        let supervisor = self.clone();
        let exited = tokio::time::timeout(SHUTDOWN_TIMEOUT, async {
            loop {
                let state = supervisor.inner.lock().await.state;
                if matches!(state, WorkerProcessState::Stopped) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;

        if exited.is_err() {
            // Force kill — drop child handle (kill_on_drop) and stdin, then mark stopped
            let mut inner = self.inner.lock().await;
            inner.stdin = None;
            inner.child_process = None;
            inner.state = WorkerProcessState::Stopped;
            log::warn!("Worker did not exit gracefully, force-stopped");
        }

        self.emit_status_changed(app, Some("shutdown".to_string()))
            .await;
        Ok(())
    }

    /// Emit a runtime:status-changed event reflecting current state.
    pub async fn emit_status_changed(&self, app: &AppHandle, reason: Option<String>) {
        self.emit_status_changed_with_error_class(app, reason, None)
            .await;
    }

    /// Emit a runtime:status-changed event with an optional load error classification.
    pub async fn emit_status_changed_with_error_class(
        &self,
        app: &AppHandle,
        reason: Option<String>,
        load_error_class: Option<String>,
    ) {
        let inner = self.inner.lock().await;
        let _ = app.emit(
            "runtime:status-changed",
            RuntimeStatusChangedEvent {
                contract_version: CONTRACT_VERSION,
                app_state: inner.state.to_app_state(),
                worker_state: inner.state.to_worker_state(),
                loaded_model_id: inner.loaded_model_id.clone(),
                reason,
                load_error_class,
            },
        );
    }

    // --- Internal ---

    async fn spawn_worker(
        &self,
        app: AppHandle,
        coordinator: JobCoordinator,
    ) -> Result<(), String> {
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<()>();

        {
            let mut inner = self.inner.lock().await;

            if !inner.worker_binary_path.exists() {
                inner.state = WorkerProcessState::Stopped;
                return Err(format!(
                    "Worker binary not found at: {}",
                    inner.worker_binary_path.display()
                ));
            }

            let mut cmd = tokio::process::Command::new(&inner.worker_binary_path);
            cmd.stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .kill_on_drop(true);

            // On Windows, prevent the worker from opening a visible console window.
            // CREATE_NO_WINDOW (0x08000000) suppresses the console while keeping
            // piped stdin/stdout/stderr functional for IPC.
            #[cfg(target_os = "windows")]
            {
                const CREATE_NO_WINDOW: u32 = 0x0800_0000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }

            let mut child = cmd
                .spawn()
                .map_err(|e| format!("Failed to spawn worker: {}", e))?;

            let stdin = child
                .stdin
                .take()
                .ok_or_else(|| "Failed to capture worker stdin".to_string())?;
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| "Failed to capture worker stdout".to_string())?;

            // Capture stderr for diagnostics (privacy-safe: only log metadata lines)
            if let Some(stderr) = child.stderr.take() {
                tokio::spawn(async move {
                    worker_stderr_reader(stderr).await;
                });
            }

            inner.state = WorkerProcessState::Starting;
            inner.stdin = Some(stdin);
            inner.loaded_model_id = None;
            inner.child_process = Some(child);

            // Spawn stdout reader task
            let supervisor_clone = self.clone();
            let coordinator_clone = coordinator.clone();
            let app_clone = app.clone();
            tokio::spawn(async move {
                worker_stdout_reader(
                    stdout,
                    supervisor_clone,
                    coordinator_clone,
                    app_clone,
                    Some(ready_tx),
                )
                .await;
            });
        }

        // Wait for ready with timeout
        match tokio::time::timeout(READY_TIMEOUT, ready_rx).await {
            Ok(Ok(())) => {
                // Worker is ready
                self.emit_status_changed(&app, None).await;
                Ok(())
            }
            _ => {
                // Timeout or channel closed before ready
                let mut inner = self.inner.lock().await;
                if inner.state == WorkerProcessState::Starting {
                    inner.state = WorkerProcessState::CrashRecovery;
                    inner.stdin = None;
                    inner.child_process = None;
                }
                drop(inner);

                self.request_restart().await;
                Err("Worker failed to become ready within timeout".to_string())
            }
        }
    }

    /// Signal the restart watcher loop to attempt a restart.
    async fn request_restart(&self) {
        let _ = self.restart_tx.send(()).await;
    }
}

/// Resolve the worker binary path by looking next to the current executable.
pub fn resolve_worker_binary_path() -> Result<PathBuf, String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current exe path: {}", e))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "Failed to get exe parent directory".to_string())?;

    let worker_name = if cfg!(windows) {
        "modutone-worker.exe"
    } else {
        "modutone-worker"
    };

    Ok(dir.join(worker_name))
}

// --- Restart watcher loop ---
// Runs as a separate tokio task. Receives restart requests via channel
// and handles the restart logic (delay, limit check, spawn).

async fn restart_watcher_loop(
    mut rx: tokio::sync::mpsc::Receiver<()>,
    supervisor: WorkerSupervisor,
    app: AppHandle,
    coordinator: JobCoordinator,
) {
    while rx.recv().await.is_some() {
        let should_restart = {
            let mut inner = supervisor.inner.lock().await;

            if inner.shutdown_requested {
                inner.state = WorkerProcessState::Stopped;
                continue;
            }

            let now = Instant::now();
            // Evict timestamps older than the restart window
            while let Some(&oldest) = inner.restart_timestamps.front() {
                if now.duration_since(oldest) > RESTART_WINDOW {
                    inner.restart_timestamps.pop_front();
                } else {
                    break;
                }
            }

            if inner.restart_timestamps.len() >= MAX_RESTARTS {
                // Restart limit exceeded
                inner.state = WorkerProcessState::Stopped;
                false
            } else {
                inner.restart_timestamps.push_back(now);
                inner.state = WorkerProcessState::CrashRecovery;
                true
            }
        };

        let restart_count = supervisor.inner.lock().await.restart_timestamps.len() as u32;

        if should_restart {
            let _ = app.emit(
                "worker:crashed",
                WorkerCrashedEvent {
                    contract_version: CONTRACT_VERSION,
                    restart_attempt: restart_count,
                    will_restart: true,
                    reason: Some("Worker process exited unexpectedly".to_string()),
                },
            );

            supervisor
                .emit_status_changed(&app, Some("Worker crashed, restarting...".to_string()))
                .await;

            // Delay before restart
            tokio::time::sleep(RESTART_DELAY).await;

            if let Err(e) = supervisor
                .spawn_worker(app.clone(), coordinator.clone())
                .await
            {
                log::error!("Failed to restart worker: {}", e);
            }
        } else {
            let _ = app.emit(
                "worker:crashed",
                WorkerCrashedEvent {
                    contract_version: CONTRACT_VERSION,
                    restart_attempt: restart_count,
                    will_restart: false,
                    reason: Some(
                        "Restart limit exceeded (3 restarts within 60 seconds)".to_string(),
                    ),
                },
            );

            supervisor
                .emit_status_changed(&app, Some("Worker restart limit exceeded".to_string()))
                .await;
        }
    }
}

// --- Background stdout reader task ---

async fn worker_stdout_reader(
    stdout: tokio::process::ChildStdout,
    supervisor: WorkerSupervisor,
    coordinator: JobCoordinator,
    app: AppHandle,
    ready_signal: Option<tokio::sync::oneshot::Sender<()>>,
) {
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut ready_signal = ready_signal;

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let message: WorkerOutbound = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to parse worker message: {}", e);
                continue;
            }
        };

        match message {
            WorkerOutbound::Ready => {
                {
                    let mut inner = supervisor.inner.lock().await;
                    if inner.state == WorkerProcessState::Starting {
                        inner.state = WorkerProcessState::Idle;
                    }
                }
                if let Some(tx) = ready_signal.take() {
                    let _ = tx.send(());
                }
            }

            WorkerOutbound::ModelLoaded {
                model_id,
                load_time_ms,
            } => {
                {
                    let mut inner = supervisor.inner.lock().await;
                    inner.loaded_model_id = Some(model_id.clone());
                    if inner.state == WorkerProcessState::Warming {
                        inner.state = WorkerProcessState::Idle;
                    }
                }
                log::info!("Model {} loaded in {}ms", model_id, load_time_ms);
                supervisor.emit_status_changed(&app, None).await;
            }

            WorkerOutbound::ModelLoadFailed { model_id, error } => {
                let error_class = classify_model_load_error(&error).to_string();
                {
                    let mut inner = supervisor.inner.lock().await;
                    // Clear loaded_model_id so supervisor state matches
                    // the worker reality (worker has no model loaded after
                    // a load failure).
                    inner.loaded_model_id = None;
                    if inner.state == WorkerProcessState::Warming {
                        inner.state = WorkerProcessState::Idle;
                    }
                }
                log::error!(
                    "Model {} load failed [{}]: {}",
                    model_id,
                    error_class,
                    error
                );
                supervisor
                    .emit_status_changed_with_error_class(
                        &app,
                        Some(format!("Model load failed: {}", error)),
                        Some(error_class),
                    )
                    .await;
            }

            // Job messages — route to coordinator
            WorkerOutbound::JobAck { job_id } => {
                coordinator.handle_job_ack(&job_id).await;
            }
            WorkerOutbound::JobProgress {
                job_id,
                partial_text,
                token_count,
            } => {
                coordinator
                    .handle_job_progress(&job_id, &partial_text, token_count, &app)
                    .await;
            }
            WorkerOutbound::JobCompleted {
                job_id,
                output_text,
                total_tokens: _,
                duration_ms: _,
            } => {
                coordinator
                    .handle_job_completed(&job_id, &output_text, &supervisor, &app)
                    .await;
            }
            WorkerOutbound::JobFailed { job_id, error } => {
                coordinator
                    .handle_job_failed(&job_id, &error, &supervisor, &app)
                    .await;
            }
            WorkerOutbound::JobCanceled { job_id } => {
                coordinator
                    .handle_job_canceled(&job_id, &supervisor, &app)
                    .await;
            }
        }
    }

    // stdout closed — worker process exited
    log::warn!("Worker stdout closed (process exited)");

    let (needs_restart, was_warming) = {
        let mut inner = supervisor.inner.lock().await;
        if inner.shutdown_requested || inner.state == WorkerProcessState::ShuttingDown {
            inner.state = WorkerProcessState::Stopped;
            inner.stdin = None;
            inner.child_process = None;
            (false, false)
        } else {
            let warming = inner.state == WorkerProcessState::Warming;
            inner.state = WorkerProcessState::CrashRecovery;
            inner.stdin = None;
            inner.child_process = None;
            (true, warming)
        }
    };

    if needs_restart {
        // If worker crashed during model load (warming), emit with
        // insufficient_memory error class so the frontend loading state
        // machine treats it as a permanent failure instead of retrying.
        if was_warming {
            log::warn!("Worker crashed during model load — likely OOM");
            supervisor
                .emit_status_changed_with_error_class(
                    &app,
                    Some("Worker crashed during model load".to_string()),
                    Some("insufficient_memory".to_string()),
                )
                .await;
        }

        // Fail any active jobs
        coordinator.handle_worker_crash(&app).await;

        // Request restart via channel (handled by restart_watcher_loop)
        supervisor.request_restart().await;
    }
}

// --- Background stderr reader task ---
// Privacy-safe: logs worker stderr lines at debug level for diagnostics.
// Does NOT log lines containing user content (prompt text, model output).
// Worker stderr is env_logger output: timestamps, log levels, module paths.

async fn worker_stderr_reader(stderr: tokio::process::ChildStderr) {
    let reader = BufReader::new(stderr);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        log::debug!("[worker stderr] {}", line);
    }
}

// --- Error Classification ---

/// Classify a model-load error string into one of three categories:
/// - `"model_invalid"`: file not found, corrupt, unsupported format
/// - `"insufficient_memory"`: out of memory, allocation failure
/// - `"transient"`: everything else (retry-eligible)
pub fn classify_model_load_error(error: &str) -> &'static str {
    let lower = error.to_lowercase();

    // Model file issues — non-retryable
    if lower.contains("not found")
        || lower.contains("no such file")
        || lower.contains("cannot find")
        || lower.contains("does not exist")
        || lower.contains("corrupt")
        || lower.contains("invalid")
        || lower.contains("unsupported")
        || lower.contains("unrecognized")
        || lower.contains("bad magic")
        || lower.contains("null result")
        || lower.contains("no module named")
        || lower.contains("mlx python runtime")
        || lower.contains("apple silicon")
    {
        return "model_invalid";
    }

    // Memory issues — non-retryable
    if lower.contains("out of memory")
        || lower.contains("alloc")
        || lower.contains("mmap")
        || lower.contains("cannot allocate")
        || lower.contains("resource temporarily unavailable")
        || lower.contains("not enough memory")
        || lower.contains("insufficient memory")
        || lower.contains("memory allocation")
        || lower.contains("enomem")
    {
        return "insufficient_memory";
    }

    // Everything else — transient / retryable
    "transient"
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_process_state_to_worker_state() {
        assert_eq!(
            WorkerProcessState::NotStarted.to_worker_state(),
            WorkerState::Unavailable
        );
        assert_eq!(
            WorkerProcessState::Starting.to_worker_state(),
            WorkerState::Unavailable
        );
        assert_eq!(
            WorkerProcessState::Idle.to_worker_state(),
            WorkerState::Idle
        );
        assert_eq!(
            WorkerProcessState::Warming.to_worker_state(),
            WorkerState::Warming
        );
        assert_eq!(
            WorkerProcessState::Busy.to_worker_state(),
            WorkerState::Busy
        );
        assert_eq!(
            WorkerProcessState::CrashRecovery.to_worker_state(),
            WorkerState::Unavailable
        );
        assert_eq!(
            WorkerProcessState::ShuttingDown.to_worker_state(),
            WorkerState::Unavailable
        );
        assert_eq!(
            WorkerProcessState::Stopped.to_worker_state(),
            WorkerState::Unavailable
        );
    }

    #[test]
    fn worker_process_state_to_app_state() {
        assert_eq!(WorkerProcessState::Idle.to_app_state(), AppState::Ready);
        assert_eq!(WorkerProcessState::Warming.to_app_state(), AppState::Ready);
        assert_eq!(WorkerProcessState::Busy.to_app_state(), AppState::Ready);
        assert_eq!(
            WorkerProcessState::NotStarted.to_app_state(),
            AppState::Degraded
        );
        assert_eq!(
            WorkerProcessState::Stopped.to_app_state(),
            AppState::Degraded
        );
        assert_eq!(
            WorkerProcessState::CrashRecovery.to_app_state(),
            AppState::Degraded
        );
    }

    #[test]
    fn resolve_worker_binary_path_returns_sibling() {
        let path = resolve_worker_binary_path();
        assert!(path.is_ok());
        let path = path.unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        if cfg!(windows) {
            assert_eq!(name, "modutone-worker.exe");
        } else {
            assert_eq!(name, "modutone-worker");
        }
    }

    #[tokio::test]
    async fn supervisor_new_starts_in_not_started() {
        let sup = WorkerSupervisor::new(PathBuf::from("/nonexistent"));
        assert_eq!(sup.get_state().await, WorkerProcessState::NotStarted);
        assert_eq!(sup.get_loaded_model_id().await, None);
    }

    #[tokio::test]
    async fn supervisor_set_busy_only_from_idle() {
        let sup = WorkerSupervisor::new(PathBuf::from("/nonexistent"));

        // NotStarted → set_busy should not change state
        sup.set_busy().await;
        assert_eq!(sup.get_state().await, WorkerProcessState::NotStarted);

        // Manually set to Idle, then set_busy should work
        sup.inner.lock().await.state = WorkerProcessState::Idle;
        sup.set_busy().await;
        assert_eq!(sup.get_state().await, WorkerProcessState::Busy);
    }

    #[tokio::test]
    async fn supervisor_transition_idle_if_busy() {
        let sup = WorkerSupervisor::new(PathBuf::from("/nonexistent"));

        // Set to Busy, then transition
        sup.inner.lock().await.state = WorkerProcessState::Busy;
        sup.transition_idle_if_busy().await;
        assert_eq!(sup.get_state().await, WorkerProcessState::Idle);

        // If not busy, should not change
        sup.inner.lock().await.state = WorkerProcessState::Warming;
        sup.transition_idle_if_busy().await;
        assert_eq!(sup.get_state().await, WorkerProcessState::Warming);
    }

    #[tokio::test]
    async fn supervisor_set_warming_only_from_idle() {
        let sup = WorkerSupervisor::new(PathBuf::from("/nonexistent"));

        // NotStarted → set_warming should fail
        assert!(sup.set_warming().await.is_err());

        // Idle → set_warming should succeed
        sup.inner.lock().await.state = WorkerProcessState::Idle;
        assert!(sup.set_warming().await.is_ok());
        assert_eq!(sup.get_state().await, WorkerProcessState::Warming);
    }

    #[tokio::test]
    async fn restart_timestamp_tracking() {
        let sup = WorkerSupervisor::new(PathBuf::from("/nonexistent"));

        {
            let mut inner = sup.inner.lock().await;
            let now = Instant::now();
            inner.restart_timestamps.push_back(now);
            inner.restart_timestamps.push_back(now);
            inner.restart_timestamps.push_back(now);
            assert_eq!(inner.restart_timestamps.len(), MAX_RESTARTS);
        }
    }

    #[tokio::test]
    async fn shutdown_when_not_started_is_noop() {
        // Cannot test shutdown fully without AppHandle,
        // but we can verify the early return condition.
        let sup = WorkerSupervisor::new(PathBuf::from("/nonexistent"));
        let state = sup.get_state().await;
        assert_eq!(state, WorkerProcessState::NotStarted);
        // shutdown would return Ok(()) for NotStarted
    }

    #[tokio::test]
    async fn state_machine_all_states_map_to_valid_worker_state() {
        let states = [
            WorkerProcessState::NotStarted,
            WorkerProcessState::Starting,
            WorkerProcessState::Idle,
            WorkerProcessState::Warming,
            WorkerProcessState::Busy,
            WorkerProcessState::CrashRecovery,
            WorkerProcessState::ShuttingDown,
            WorkerProcessState::Stopped,
        ];

        for state in states {
            // Should not panic
            let _ = state.to_worker_state();
            let _ = state.to_app_state();
        }
    }

    #[test]
    fn classify_model_load_error_model_invalid() {
        assert_eq!(classify_model_load_error("file not found"), "model_invalid");
        assert_eq!(
            classify_model_load_error("No such file or directory"),
            "model_invalid"
        );
        assert_eq!(
            classify_model_load_error("Cannot find the specified path"),
            "model_invalid"
        );
        assert_eq!(
            classify_model_load_error("Model path does not exist"),
            "model_invalid"
        );
        assert_eq!(
            classify_model_load_error("File is corrupt or truncated"),
            "model_invalid"
        );
        assert_eq!(
            classify_model_load_error("Invalid GGUF header"),
            "model_invalid"
        );
        assert_eq!(
            classify_model_load_error("Unsupported model format"),
            "model_invalid"
        );
        assert_eq!(
            classify_model_load_error("Unrecognized quantization type"),
            "model_invalid"
        );
        assert_eq!(
            classify_model_load_error("bad magic number in header"),
            "model_invalid"
        );
        assert_eq!(
            classify_model_load_error(
                "Failed to load model from '/path/model.gguf': null result from llama cpp"
            ),
            "model_invalid"
        );
    }

    #[test]
    fn classify_model_load_error_insufficient_memory() {
        assert_eq!(
            classify_model_load_error("out of memory"),
            "insufficient_memory"
        );
        assert_eq!(
            classify_model_load_error("Failed to alloc buffer"),
            "insufficient_memory"
        );
        assert_eq!(
            classify_model_load_error("mmap failed: cannot allocate memory"),
            "insufficient_memory"
        );
        assert_eq!(
            classify_model_load_error("cannot allocate 8GB"),
            "insufficient_memory"
        );
        assert_eq!(
            classify_model_load_error("resource temporarily unavailable"),
            "insufficient_memory"
        );
        assert_eq!(
            classify_model_load_error("Not enough memory to complete this operation"),
            "insufficient_memory"
        );
        assert_eq!(
            classify_model_load_error("Insufficient memory for model"),
            "insufficient_memory"
        );
        assert_eq!(
            classify_model_load_error("memory allocation failed at 0x7fff"),
            "insufficient_memory"
        );
        assert_eq!(
            classify_model_load_error("ENOMEM: failed to map file"),
            "insufficient_memory"
        );
    }

    #[test]
    fn classify_model_load_error_transient() {
        assert_eq!(
            classify_model_load_error("unknown error occurred"),
            "transient"
        );
        assert_eq!(
            classify_model_load_error("backend initialization failed"),
            "transient"
        );
        assert_eq!(classify_model_load_error("lock poisoned"), "transient");
        assert_eq!(classify_model_load_error(""), "transient");
    }
}
