// Phase: 9
// ModuTone inference worker entry point
//
// This is a genuinely separate binary that communicates with the
// Tauri backend via stdin/stdout JSON Lines protocol.
// It does NOT link against the Tauri backend crate.
//
// Architecture:
//   [Stdin Reader Thread] --event--> [Main Thread Event Loop] --stdout-->
//   [Inference Thread]    --event--> [Main Thread Event Loop]
//
// The loaded model adapter is held as Arc<dyn ModelAdapter> for
// thread-safe sharing with inference threads.

mod adapter;
mod cancellation;
mod protocol;

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Instant;

use adapter::llama_cpp::LlamaCppAdapter;
use adapter::ModelAdapter;
use cancellation::CancellationToken;
use protocol::{InboundMessage, OutboundMessage};

/// Events processed by the main thread event loop.
enum WorkerEvent {
    /// Message received from stdin (backend).
    Stdin(InboundMessage),
    /// Stdin reader encountered an error or EOF.
    StdinClosed,
    /// Inference progress update from a running job.
    InferenceProgress {
        job_id: String,
        partial_text: String,
        token_count: u32,
    },
    /// Inference completed (success or failure).
    InferenceComplete {
        job_id: String,
        result: Result<adapter::GenerationResult, String>,
        was_canceled: bool,
    },
}

fn main() {
    env_logger::init();

    let stdout = io::stdout();

    // Event channel: all threads send events here, main thread receives.
    let (event_tx, event_rx) = mpsc::channel::<WorkerEvent>();

    // Track active job cancellation tokens by job_id
    let mut active_jobs: HashMap<String, CancellationToken> = HashMap::new();

    // Currently loaded model adapter (thread-safe shared reference)
    let mut loaded_model: Option<Arc<dyn ModelAdapter>> = None;

    // Send ready message
    send_message(&stdout, &OutboundMessage::Ready);

    // Spawn stdin reader thread
    let stdin_tx = event_tx.clone();
    std::thread::spawn(move || {
        stdin_reader_loop(stdin_tx);
    });

    // Main event loop
    for event in event_rx {
        match event {
            WorkerEvent::Stdin(message) => match message {
                InboundMessage::Shutdown => {
                    log::info!("Received shutdown signal, exiting");
                    break;
                }

                InboundMessage::LoadModel {
                    model_id,
                    model_path,
                } => {
                    // Load model synchronously on main thread.
                    // This blocks the event loop for a few seconds, which is
                    // acceptable because the backend's state machine already
                    // accounts for model loading latency (Warming state).
                    let start = Instant::now();
                    log::info!(
                        "Loading model: id={}, path_len={}",
                        model_id,
                        model_path.len()
                    );

                    // Drop old model (and its LlamaBackend) before loading
                    // the new one. The llama.cpp backend is a global singleton:
                    // init fails with BackendAlreadyInitialized if the old
                    // backend hasn't been freed yet.
                    drop(loaded_model.take());

                    match LlamaCppAdapter::load(&model_path, &model_id) {
                        Ok(adapter) => {
                            let load_time_ms = start.elapsed().as_millis() as u64;
                            loaded_model = Some(Arc::new(adapter));
                            send_message(
                                &stdout,
                                &OutboundMessage::ModelLoaded {
                                    model_id,
                                    load_time_ms,
                                },
                            );
                        }
                        Err(error) => {
                            log::error!("Model load failed for {}: {}", model_id, error);
                            loaded_model = None;
                            send_message(
                                &stdout,
                                &OutboundMessage::ModelLoadFailed { model_id, error },
                            );
                        }
                    }
                }

                InboundMessage::ExecuteJob {
                    job_id,
                    prompt_package,
                } => {
                    // Reject if another job is still active (defense against
                    // race after cancel_ack_timeout on the backend side)
                    if !active_jobs.is_empty() {
                        send_message(
                            &stdout,
                            &OutboundMessage::JobFailed {
                                job_id,
                                error: "Worker already has an active job".to_string(),
                            },
                        );
                        continue;
                    }

                    // Send ack immediately
                    send_message(
                        &stdout,
                        &OutboundMessage::JobAck {
                            job_id: job_id.clone(),
                        },
                    );

                    let adapter = match loaded_model.clone() {
                        Some(a) => a,
                        None => {
                            send_message(
                                &stdout,
                                &OutboundMessage::JobFailed {
                                    job_id,
                                    error: "No model loaded".to_string(),
                                },
                            );
                            continue;
                        }
                    };

                    // Create cancellation token for this job
                    let token = CancellationToken::new();
                    active_jobs.insert(job_id.clone(), token.clone());

                    // Spawn inference thread
                    let tx = event_tx.clone();
                    let jid = job_id.clone();
                    let cancel = token.clone();
                    std::thread::spawn(move || {
                        run_inference(tx, jid, adapter, prompt_package, cancel);
                    });
                }

                InboundMessage::CancelJob { job_id } => {
                    log::info!("cancel_job for job_id={}", job_id);
                    if let Some(token) = active_jobs.get(&job_id) {
                        token.cancel();
                    }
                    // Note: JobCanceled will be sent when the inference thread
                    // detects cancellation and sends InferenceComplete.
                    // If no active job, send canceled immediately.
                    if !active_jobs.contains_key(&job_id) {
                        send_message(&stdout, &OutboundMessage::JobCanceled { job_id });
                    }
                }
            },

            WorkerEvent::StdinClosed => {
                log::info!("Stdin closed, exiting");
                break;
            }

            WorkerEvent::InferenceProgress {
                job_id,
                partial_text,
                token_count,
            } => {
                send_message(
                    &stdout,
                    &OutboundMessage::JobProgress {
                        job_id,
                        partial_text,
                        token_count,
                    },
                );
            }

            WorkerEvent::InferenceComplete {
                job_id,
                result,
                was_canceled,
            } => {
                active_jobs.remove(&job_id);

                if was_canceled {
                    send_message(
                        &stdout,
                        &OutboundMessage::JobCanceled {
                            job_id: job_id.clone(),
                        },
                    );
                } else {
                    match result {
                        Ok(gen_result) => {
                            send_message(
                                &stdout,
                                &OutboundMessage::JobCompleted {
                                    job_id: job_id.clone(),
                                    output_text: gen_result.output_text,
                                    total_tokens: gen_result.total_tokens,
                                    duration_ms: gen_result.duration_ms,
                                },
                            );
                        }
                        Err(error) => {
                            send_message(
                                &stdout,
                                &OutboundMessage::JobFailed {
                                    job_id: job_id.clone(),
                                    error,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Stdin reader loop — runs on a dedicated thread.
/// Reads JSON Lines from stdin, parses them, and sends events to the main thread.
fn stdin_reader_loop(tx: mpsc::Sender<WorkerEvent>) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to read stdin: {}", e);
                let _ = tx.send(WorkerEvent::StdinClosed);
                return;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let message: InboundMessage = match serde_json::from_str(&line) {
            Ok(m) => m,
            Err(e) => {
                log::error!("Failed to parse message: {}", e);
                continue;
            }
        };

        if tx.send(WorkerEvent::Stdin(message)).is_err() {
            // Main thread has exited
            return;
        }
    }

    // stdin closed (EOF)
    let _ = tx.send(WorkerEvent::StdinClosed);
}

/// Run inference on a dedicated thread.
/// Sends progress and completion events back to the main thread.
fn run_inference(
    tx: mpsc::Sender<WorkerEvent>,
    job_id: String,
    adapter: Arc<dyn ModelAdapter>,
    prompt_package: protocol::PromptPackage,
    cancel: CancellationToken,
) {
    let jid_progress = job_id.clone();
    let tx_progress = tx.clone();

    let progress_cb = move |partial_text: &str, token_count: u32| {
        let _ = tx_progress.send(WorkerEvent::InferenceProgress {
            job_id: jid_progress.clone(),
            partial_text: partial_text.to_string(),
            token_count,
        });
    };

    let result = adapter.generate(&prompt_package, &cancel, &progress_cb);
    let was_canceled = cancel.is_canceled();

    let _ = tx.send(WorkerEvent::InferenceComplete {
        job_id,
        result,
        was_canceled,
    });
}

fn send_message(stdout: &io::Stdout, message: &OutboundMessage) {
    let mut handle = stdout.lock();
    if let Ok(json) = serde_json::to_string(message) {
        let _ = writeln!(handle, "{}", json);
        let _ = handle.flush();
    }
}
