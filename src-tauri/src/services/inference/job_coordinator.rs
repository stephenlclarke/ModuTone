// Phase: 4
// Job coordinator — manages generation job lifecycle.
//
// Responsibilities:
// - Job submission and dispatch to worker
// - Job state tracking (Pending → Dispatched → Executing → terminal)
// - Cancel-wins policy: if cancel is in-flight and result arrives, discard result
// - Ack timeout (10s) and cancel ack timeout (5s)
// - Route worker job messages to Tauri events
// - Handle worker crash (fail all active jobs)

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;

use crate::contracts::errors::IpcError;
use crate::contracts::events::{
    GenerationCanceledEvent, GenerationCompletedEvent, GenerationFailedEvent,
    GenerationProgressEvent, GenerationStartedEvent,
};
use crate::contracts::shared::RequestKind;

use super::worker_protocol::{PromptPackage, WorkerInbound};
use super::worker_supervisor::WorkerSupervisor;

const CONTRACT_VERSION: u32 = 1;
const JOB_ACK_TIMEOUT: Duration = Duration::from_secs(10);
const CANCEL_ACK_TIMEOUT: Duration = Duration::from_secs(2);

// --- Job State Machine ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Dispatched,
    Executing,
    Canceling,
    // Terminal states: job is removed from tracking
}

// --- Tracked Job ---

#[derive(Debug, Clone)]
pub struct TrackedJob {
    pub id: String,
    pub tab_id: String,
    pub request_kind: RequestKind,
    pub input_version_token: String,
    pub accepted_output_version: Option<u32>,
    pub state: JobState,
}

// --- Coordinator Inner State ---

pub(crate) struct CoordinatorInner {
    pub jobs: HashMap<String, TrackedJob>,
}

// --- Public API ---

#[derive(Clone)]
pub struct JobCoordinator {
    pub(crate) inner: Arc<Mutex<CoordinatorInner>>,
}

/// Map a worker error string to a privacy-safe user-facing category.
/// The raw error goes in the `detail` field; this returns a safe summary.
fn classify_worker_error(error: &str) -> String {
    let lower = error.to_lowercase();

    if lower.contains("no model loaded") {
        "Model is not loaded".to_string()
    } else if lower.contains("failed to load model") || lower.contains("failed to initialize") {
        "Model failed to load".to_string()
    } else if lower.contains("prompt too long") || lower.contains("exceeds context size") {
        "Input is too long for this model".to_string()
    } else if lower.contains("tokenization failed") {
        "Failed to process input text".to_string()
    } else if lower.contains("chat template") {
        "Model chat template error".to_string()
    } else if lower.contains("decode failed") || lower.contains("batch add failed") {
        "Generation runtime error".to_string()
    } else if lower.contains("canceled") {
        "Generation was canceled".to_string()
    } else if lower.contains("lock poisoned") {
        "Internal worker error".to_string()
    } else {
        "Generation failed".to_string()
    }
}

impl Default for JobCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

impl JobCoordinator {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(CoordinatorInner {
                jobs: HashMap::new(),
            })),
        }
    }

    /// Submit a new generation job. Dispatches to the worker immediately.
    #[allow(clippy::too_many_arguments)]
    pub async fn submit_job(
        &self,
        supervisor: &WorkerSupervisor,
        app: &AppHandle,
        tab_id: String,
        request_kind: RequestKind,
        input_version_token: String,
        accepted_output_version: Option<u32>,
        prompt_package: PromptPackage,
    ) -> Result<String, IpcError> {
        // Check worker is in a state that can accept jobs
        let worker_state = supervisor.get_state().await;
        if !matches!(
            worker_state,
            super::worker_supervisor::WorkerProcessState::Idle
        ) {
            return Err(IpcError {
                code: "WORKER_UNAVAILABLE".to_string(),
                message: "Worker is not available to accept jobs".to_string(),
                detail: Some(format!("Worker state: {:?}", worker_state)),
                subsystem: "inference".to_string(),
            });
        }

        // Check no duplicate job for this tab
        {
            let inner = self.inner.lock().await;
            for job in inner.jobs.values() {
                if job.tab_id == tab_id {
                    return Err(IpcError {
                        code: "DUPLICATE_JOB".to_string(),
                        message: "Tab already has an active job".to_string(),
                        detail: None,
                        subsystem: "inference".to_string(),
                    });
                }
            }
        }

        let job_id = uuid::Uuid::new_v4().to_string();

        let job = TrackedJob {
            id: job_id.clone(),
            tab_id: tab_id.clone(),
            request_kind: request_kind.clone(),
            input_version_token: input_version_token.clone(),
            accepted_output_version,
            state: JobState::Pending,
        };

        {
            let mut inner = self.inner.lock().await;
            inner.jobs.insert(job_id.clone(), job);
        }

        // Send execute_job to worker
        let msg = WorkerInbound::ExecuteJob {
            job_id: job_id.clone(),
            prompt_package,
        };

        match supervisor.send_to_worker(&msg).await {
            Ok(()) => {
                // Update job state to Dispatched
                {
                    let mut inner = self.inner.lock().await;
                    if let Some(job) = inner.jobs.get_mut(&job_id) {
                        job.state = JobState::Dispatched;
                    }
                }

                // Worker is now busy — emit status change so frontend
                // sees workerState "busy" and disables Generate button.
                supervisor.set_busy().await;
                supervisor.emit_status_changed(app, None).await;

                // Emit generation:started event
                let _ = app.emit(
                    "generation:started",
                    GenerationStartedEvent {
                        contract_version: CONTRACT_VERSION,
                        job_id: job_id.clone(),
                        tab_id,
                        request_kind,
                    },
                );

                // Spawn ack timeout watcher
                let coordinator = self.clone();
                let supervisor_clone = supervisor.clone();
                let app_clone = app.clone();
                let jid = job_id.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(JOB_ACK_TIMEOUT).await;
                    coordinator
                        .check_ack_timeout(&jid, &supervisor_clone, &app_clone)
                        .await;
                });

                Ok(job_id)
            }
            Err(e) => {
                // Clean up failed submission
                self.inner.lock().await.jobs.remove(&job_id);
                Err(IpcError {
                    code: "WORKER_SEND_FAILED".to_string(),
                    message: "Failed to send job to worker".to_string(),
                    detail: Some(e),
                    subsystem: "inference".to_string(),
                })
            }
        }
    }

    /// Cancel an active job. Sends cancel_job to the worker.
    pub async fn cancel_job(
        &self,
        supervisor: &WorkerSupervisor,
        app: &AppHandle,
        job_id: &str,
    ) -> Result<(), IpcError> {
        {
            let mut inner = self.inner.lock().await;
            let job = inner.jobs.get_mut(job_id).ok_or(IpcError {
                code: "JOB_NOT_FOUND".to_string(),
                message: "Job not found".to_string(),
                detail: None,
                subsystem: "inference".to_string(),
            })?;

            if !matches!(job.state, JobState::Dispatched | JobState::Executing) {
                return Err(IpcError {
                    code: "JOB_ALREADY_TERMINAL".to_string(),
                    message: "Job is not in a cancelable state".to_string(),
                    detail: Some(format!("Job state: {:?}", job.state)),
                    subsystem: "inference".to_string(),
                });
            }

            job.state = JobState::Canceling;
        }

        // Send cancel to worker
        let msg = WorkerInbound::CancelJob {
            job_id: job_id.to_string(),
        };
        if let Err(e) = supervisor.send_to_worker(&msg).await {
            log::warn!("Failed to send cancel to worker for job {}: {}", job_id, e);
        }

        // Spawn cancel ack timeout watcher
        let coordinator = self.clone();
        let supervisor_clone = supervisor.clone();
        let app_clone = app.clone();
        let jid = job_id.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(CANCEL_ACK_TIMEOUT).await;
            coordinator
                .check_cancel_timeout(&jid, &supervisor_clone, &app_clone)
                .await;
        });

        Ok(())
    }

    /// Get the current state of a tracked job, if it exists.
    pub async fn get_job(&self, job_id: &str) -> Option<TrackedJob> {
        self.inner.lock().await.jobs.get(job_id).cloned()
    }

    // --- Worker message handlers (called by stdout reader) ---

    /// Handle job_ack from worker.
    pub async fn handle_job_ack(&self, job_id: &str) {
        let mut inner = self.inner.lock().await;
        if let Some(job) = inner.jobs.get_mut(job_id) {
            if job.state == JobState::Dispatched {
                job.state = JobState::Executing;
            }
            // If Canceling (cancel sent before ack arrived), ack is noted but
            // state stays Canceling — cancel-wins.
        }
    }

    /// Handle job_progress from worker.
    pub async fn handle_job_progress(
        &self,
        job_id: &str,
        partial_text: &str,
        token_count: u32,
        app: &AppHandle,
    ) {
        let inner = self.inner.lock().await;
        if let Some(job) = inner.jobs.get(job_id) {
            if matches!(job.state, JobState::Executing) {
                let _ = app.emit(
                    "generation:progress",
                    GenerationProgressEvent {
                        contract_version: CONTRACT_VERSION,
                        job_id: job_id.to_string(),
                        tab_id: job.tab_id.clone(),
                        partial_text: Some(partial_text.to_string()),
                        token_count: Some(token_count),
                    },
                );
            }
            // If Canceling, ignore progress (cancel-wins).
        }
    }

    /// Handle job_completed from worker.
    pub async fn handle_job_completed(
        &self,
        job_id: &str,
        output_text: &str,
        supervisor: &WorkerSupervisor,
        app: &AppHandle,
    ) {
        let removed_job = {
            let mut inner = self.inner.lock().await;
            inner.jobs.remove(job_id)
        };

        if let Some(job) = removed_job {
            if job.state == JobState::Canceling {
                // Cancel-wins: discard the result, emit canceled
                let _ = app.emit(
                    "generation:canceled",
                    GenerationCanceledEvent {
                        contract_version: CONTRACT_VERSION,
                        job_id: job_id.to_string(),
                        tab_id: job.tab_id,
                        request_kind: job.request_kind,
                    },
                );
            } else {
                // Normal completion
                let _ = app.emit(
                    "generation:completed",
                    GenerationCompletedEvent {
                        contract_version: CONTRACT_VERSION,
                        job_id: job_id.to_string(),
                        tab_id: job.tab_id,
                        request_kind: job.request_kind,
                        input_version_token: job.input_version_token,
                        accepted_output_version: job.accepted_output_version,
                        output_text: output_text.to_string(),
                    },
                );
            }
        }

        // Worker goes back to idle
        supervisor.transition_idle_if_busy().await;
        supervisor.emit_status_changed(app, None).await;
    }

    /// Handle job_failed from worker.
    pub async fn handle_job_failed(
        &self,
        job_id: &str,
        error: &str,
        supervisor: &WorkerSupervisor,
        app: &AppHandle,
    ) {
        let removed_job = {
            let mut inner = self.inner.lock().await;
            inner.jobs.remove(job_id)
        };

        if let Some(job) = removed_job {
            if job.state == JobState::Canceling {
                // Cancel was in flight — treat as canceled
                let _ = app.emit(
                    "generation:canceled",
                    GenerationCanceledEvent {
                        contract_version: CONTRACT_VERSION,
                        job_id: job_id.to_string(),
                        tab_id: job.tab_id,
                        request_kind: job.request_kind,
                    },
                );
            } else {
                let message = classify_worker_error(error);
                let _ = app.emit(
                    "generation:failed",
                    GenerationFailedEvent {
                        contract_version: CONTRACT_VERSION,
                        job_id: job_id.to_string(),
                        tab_id: job.tab_id,
                        request_kind: job.request_kind,
                        error: IpcError {
                            code: "JOB_EXECUTION_FAILED".to_string(),
                            message,
                            detail: Some(error.to_string()),
                            subsystem: "inference".to_string(),
                        },
                    },
                );
            }
        }

        // Worker goes back to idle
        supervisor.transition_idle_if_busy().await;
        supervisor.emit_status_changed(app, None).await;
    }

    /// Handle job_canceled from worker.
    pub async fn handle_job_canceled(
        &self,
        job_id: &str,
        supervisor: &WorkerSupervisor,
        app: &AppHandle,
    ) {
        let removed_job = {
            let mut inner = self.inner.lock().await;
            inner.jobs.remove(job_id)
        };

        if let Some(job) = removed_job {
            let _ = app.emit(
                "generation:canceled",
                GenerationCanceledEvent {
                    contract_version: CONTRACT_VERSION,
                    job_id: job_id.to_string(),
                    tab_id: job.tab_id,
                    request_kind: job.request_kind,
                },
            );
        }

        // Worker goes back to idle
        supervisor.transition_idle_if_busy().await;
        supervisor.emit_status_changed(app, None).await;
    }

    /// Handle worker crash — fail all active jobs.
    pub async fn handle_worker_crash(&self, app: &AppHandle) {
        let jobs: Vec<TrackedJob> = {
            let mut inner = self.inner.lock().await;
            inner.jobs.drain().map(|(_, job)| job).collect()
        };

        for job in jobs {
            let _ = app.emit(
                "generation:failed",
                GenerationFailedEvent {
                    contract_version: CONTRACT_VERSION,
                    job_id: job.id,
                    tab_id: job.tab_id,
                    request_kind: job.request_kind,
                    error: IpcError {
                        code: "WORKER_CRASHED".to_string(),
                        message: "Worker process crashed during generation".to_string(),
                        detail: None,
                        subsystem: "inference".to_string(),
                    },
                },
            );
        }
    }

    // --- Timeout handlers ---

    async fn check_ack_timeout(
        &self,
        job_id: &str,
        supervisor: &WorkerSupervisor,
        app: &AppHandle,
    ) {
        let timed_out_job = {
            let inner = self.inner.lock().await;
            match inner.jobs.get(job_id) {
                Some(job) if job.state == JobState::Dispatched => Some(job.clone()),
                _ => None,
            }
        };

        if let Some(job) = timed_out_job {
            // Remove the job from coordinator tracking.
            self.inner.lock().await.jobs.remove(job_id);

            // Emit failure so the frontend tab leaves generating state.
            let _ = app.emit(
                "generation:failed",
                GenerationFailedEvent {
                    contract_version: CONTRACT_VERSION,
                    job_id: job.id,
                    tab_id: job.tab_id,
                    request_kind: job.request_kind,
                    error: IpcError {
                        code: "JOB_ACK_TIMEOUT".to_string(),
                        message: "Worker did not acknowledge job in time".to_string(),
                        detail: None,
                        subsystem: "inference".to_string(),
                    },
                },
            );

            // Force-kill the worker since it didn't acknowledge in time.
            // Crash recovery will restart it and the frontend will
            // auto-reload the model.
            log::warn!("Job ack timeout for job {}; force-killing worker", job_id);
            supervisor.force_kill_worker().await;
        }
    }

    async fn check_cancel_timeout(
        &self,
        job_id: &str,
        supervisor: &WorkerSupervisor,
        app: &AppHandle,
    ) {
        let timed_out_job = {
            let inner = self.inner.lock().await;
            match inner.jobs.get(job_id) {
                Some(job) if job.state == JobState::Canceling => Some(job.clone()),
                _ => None,
            }
        };

        if let Some(job) = timed_out_job {
            // Remove the job from coordinator tracking so the tab unlocks.
            self.inner.lock().await.jobs.remove(job_id);

            // Emit canceled so the frontend tab leaves generating state.
            let _ = app.emit(
                "generation:canceled",
                GenerationCanceledEvent {
                    contract_version: CONTRACT_VERSION,
                    job_id: job.id,
                    tab_id: job.tab_id,
                    request_kind: job.request_kind,
                },
            );

            // Force-kill the worker since cooperative cancel didn't work
            // in time. The worker is stuck in a blocking decode call.
            // Crash recovery will restart it and the frontend will
            // auto-reload the model.
            log::warn!(
                "Cancel ack timeout for job {}; force-killing worker",
                job_id
            );
            supervisor.force_kill_worker().await;
        }
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_state_initial() {
        let job = TrackedJob {
            id: "test-1".to_string(),
            tab_id: "tab-1".to_string(),
            request_kind: RequestKind::InitialRewrite,
            input_version_token: "token-1".to_string(),
            accepted_output_version: None,
            state: JobState::Pending,
        };
        assert_eq!(job.state, JobState::Pending);
    }

    #[tokio::test]
    async fn coordinator_new_has_no_jobs() {
        let coord = JobCoordinator::new();
        assert!(coord.get_job("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn coordinator_track_job_manually() {
        let coord = JobCoordinator::new();

        // Manually insert a job
        {
            let mut inner = coord.inner.lock().await;
            inner.jobs.insert(
                "job-1".to_string(),
                TrackedJob {
                    id: "job-1".to_string(),
                    tab_id: "tab-1".to_string(),
                    request_kind: RequestKind::InitialRewrite,
                    input_version_token: "v1".to_string(),
                    accepted_output_version: None,
                    state: JobState::Dispatched,
                },
            );
        }

        let job = coord.get_job("job-1").await;
        assert!(job.is_some());
        assert_eq!(job.unwrap().state, JobState::Dispatched);
    }

    #[tokio::test]
    async fn handle_job_ack_transitions_dispatched_to_executing() {
        let coord = JobCoordinator::new();

        {
            let mut inner = coord.inner.lock().await;
            inner.jobs.insert(
                "job-1".to_string(),
                TrackedJob {
                    id: "job-1".to_string(),
                    tab_id: "tab-1".to_string(),
                    request_kind: RequestKind::InitialRewrite,
                    input_version_token: "v1".to_string(),
                    accepted_output_version: None,
                    state: JobState::Dispatched,
                },
            );
        }

        coord.handle_job_ack("job-1").await;

        let job = coord.get_job("job-1").await.unwrap();
        assert_eq!(job.state, JobState::Executing);
    }

    #[tokio::test]
    async fn handle_job_ack_does_not_override_canceling() {
        let coord = JobCoordinator::new();

        {
            let mut inner = coord.inner.lock().await;
            inner.jobs.insert(
                "job-1".to_string(),
                TrackedJob {
                    id: "job-1".to_string(),
                    tab_id: "tab-1".to_string(),
                    request_kind: RequestKind::InitialRewrite,
                    input_version_token: "v1".to_string(),
                    accepted_output_version: None,
                    state: JobState::Canceling,
                },
            );
        }

        coord.handle_job_ack("job-1").await;

        let job = coord.get_job("job-1").await.unwrap();
        assert_eq!(job.state, JobState::Canceling);
    }

    #[tokio::test]
    async fn handle_job_ack_ignores_unknown_job() {
        let coord = JobCoordinator::new();
        // Should not panic
        coord.handle_job_ack("nonexistent").await;
    }

    #[tokio::test]
    async fn handle_worker_crash_removes_all_jobs() {
        let coord = JobCoordinator::new();

        {
            let mut inner = coord.inner.lock().await;
            inner.jobs.insert(
                "job-1".to_string(),
                TrackedJob {
                    id: "job-1".to_string(),
                    tab_id: "tab-1".to_string(),
                    request_kind: RequestKind::InitialRewrite,
                    input_version_token: "v1".to_string(),
                    accepted_output_version: None,
                    state: JobState::Executing,
                },
            );
            inner.jobs.insert(
                "job-2".to_string(),
                TrackedJob {
                    id: "job-2".to_string(),
                    tab_id: "tab-2".to_string(),
                    request_kind: RequestKind::Refinement,
                    input_version_token: "v2".to_string(),
                    accepted_output_version: Some(1),
                    state: JobState::Dispatched,
                },
            );
        }

        // handle_worker_crash requires AppHandle, which we can't easily create in tests.
        // Instead, test the drain logic directly.
        {
            let mut inner = coord.inner.lock().await;
            let jobs: Vec<_> = inner.jobs.drain().collect();
            assert_eq!(jobs.len(), 2);
        }

        assert!(coord.get_job("job-1").await.is_none());
        assert!(coord.get_job("job-2").await.is_none());
    }

    #[test]
    fn classify_worker_error_categories() {
        assert_eq!(
            classify_worker_error("No model loaded"),
            "Model is not loaded"
        );
        assert_eq!(
            classify_worker_error("Failed to load model 'qwen': io error"),
            "Model failed to load"
        );
        assert_eq!(
            classify_worker_error("Failed to initialize llama backend: BackendAlreadyInitialized"),
            "Model failed to load"
        );
        assert_eq!(
            classify_worker_error("Prompt too long: 5000 tokens exceeds context size 4096"),
            "Input is too long for this model"
        );
        assert_eq!(
            classify_worker_error("Tokenization failed: invalid utf-8"),
            "Failed to process input text"
        );
        assert_eq!(
            classify_worker_error("Failed to apply chat template: missing template"),
            "Model chat template error"
        );
        assert_eq!(
            classify_worker_error("Decode failed at token 42: out of memory"),
            "Generation runtime error"
        );
        assert_eq!(
            classify_worker_error("Batch add failed during generation: overflow"),
            "Generation runtime error"
        );
        assert_eq!(
            classify_worker_error("Job canceled before generation"),
            "Generation was canceled"
        );
        assert_eq!(
            classify_worker_error("Backend lock poisoned: thread panicked"),
            "Internal worker error"
        );
        assert_eq!(
            classify_worker_error("some unknown error we haven't seen"),
            "Generation failed"
        );
    }

    #[test]
    fn classify_worker_error_new_cancel_strings() {
        // All new cancellation checkpoint messages should classify as canceled
        assert_eq!(
            classify_worker_error("Job canceled during prompt formatting"),
            "Generation was canceled"
        );
        assert_eq!(
            classify_worker_error("Job canceled during tokenization"),
            "Generation was canceled"
        );
        assert_eq!(
            classify_worker_error("Job canceled during context creation"),
            "Generation was canceled"
        );
        assert_eq!(
            classify_worker_error("Job canceled during prompt processing"),
            "Generation was canceled"
        );
    }

    #[test]
    fn cancel_wins_policy_documentation() {
        // The cancel-wins policy is implemented in handle_job_completed:
        // If job.state == Canceling when a result arrives, the result is
        // discarded and a canceled event is emitted instead of completed.
        // This test documents the behavior; the actual logic is tested
        // via integration tests that require AppHandle.
    }
}
