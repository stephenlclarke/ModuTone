// MLX adapter for Apple Silicon model directories.
//
// This backend keeps ModuTone's worker protocol stable while delegating MLX
// execution to a small Python bridge. Prompt text is sent over stdin, not as a
// process argument, so it is not exposed through process listings.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::json;

use crate::cancellation::CancellationToken;
use crate::protocol::PromptPackage;

use super::{GenerationResult, ModelAdapter};

const PYTHON_BRIDGE: &str = r#"
import json
import queue
import re
import sys
import threading
import traceback

def send(message):
    print(json.dumps(message, ensure_ascii=False), flush=True)

def format_prompt(tokenizer, system_prompt, user_message):
    messages = [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_message},
    ]
    if hasattr(tokenizer, "apply_chat_template"):
        try:
            return tokenizer.apply_chat_template(
                messages,
                add_generation_prompt=True,
                reasoning_effort="low",
            )
        except TypeError:
            return tokenizer.apply_chat_template(messages, add_generation_prompt=True)
    return (
        "System:\n" + system_prompt
        + "\n\nUser:\n" + user_message
        + "\n\nAssistant:\n"
    )

def clean_output(text):
    final_matches = list(
        re.finditer(r"<\|channel\|>?final<\|message\|>?", text)
    )
    if final_matches:
        text = text[final_matches[-1].end():]
    elif re.search(r"<\|channel\|>?analysis<\|message\|>?", text):
        return ""

    text = re.split(
        r"<\|end\|>|<\|start\|>|<\|channel\|>?analysis<\|message\|>?",
        text,
        maxsplit=1,
    )[0]
    text = re.sub(r"<\|channel\|>?(analysis|final)?", "", text)
    text = re.sub(r"<\|message\|>?", "", text)
    text = re.sub(r"<\|[^<\s]*?\|>?", "", text)
    return text.strip()

try:
    from mlx_lm import stream_generate
    from mlx_lm.models.cache import make_prompt_cache
    from mlx_lm.sample_utils import make_logits_processors, make_sampler
    from turboquant_mlx.generate import load_turboquant

    model_path = sys.argv[1]
    model, tokenizer = load_turboquant(model_path)
    send({"type": "ready"})
except Exception as exc:
    send({"type": "load_failed", "error": str(exc)})
    sys.exit(2)

request_queue = queue.Queue()
cancel_event = threading.Event()
shutdown_event = threading.Event()

def stdin_reader():
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            request = json.loads(line)
        except Exception:
            send({
                "type": "failed",
                "error": "Invalid bridge control message",
                "trace": None,
            })
            continue

        request_type = request.get("type")
        if request_type == "cancel":
            cancel_event.set()
            continue
        if request_type == "shutdown":
            shutdown_event.set()
            cancel_event.set()
            request_queue.put(request)
            break

        request_queue.put(request)

threading.Thread(target=stdin_reader, daemon=True).start()

while not shutdown_event.is_set():
    try:
        request = request_queue.get()
        if request.get("type") == "shutdown":
            break
        if request.get("type") != "generate":
            continue

        cancel_event.clear()

        prompt = format_prompt(
            tokenizer,
            request["system_prompt"],
            request["user_message"],
        )
        prompt_cache = make_prompt_cache(model)
        sampler = make_sampler(temp=float(request["temperature"]))
        output_parts = []
        total_tokens = 0
        canceled = False
        for response in stream_generate(
            model,
            tokenizer,
            prompt=prompt,
            max_tokens=int(request["max_tokens"]),
            sampler=sampler,
            logits_processors=make_logits_processors(repetition_penalty=1.1),
            prompt_cache=prompt_cache,
        ):
            if cancel_event.is_set() or shutdown_event.is_set():
                canceled = True
                break
            output_parts.append(response.text)
            total_tokens = int(getattr(response, "generation_tokens", total_tokens))

        if canceled:
            send({"type": "canceled"})
            if shutdown_event.is_set():
                break
            continue

        text = clean_output("".join(output_parts))
        send({
            "type": "completed",
            "output_text": text,
            "total_tokens": total_tokens,
        })
    except Exception as exc:
        send({
            "type": "failed",
            "error": str(exc),
            "trace": traceback.format_exc(limit=2),
        })
"#;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BridgeMessage {
    Ready,
    LoadFailed {
        error: String,
    },
    Completed {
        output_text: String,
        total_tokens: u32,
    },
    Canceled,
    Failed {
        error: String,
        #[allow(dead_code)]
        trace: Option<String>,
    },
}

struct BridgeProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Drop for BridgeProcess {
    fn drop(&mut self) {
        let _ = writeln!(self.stdin, r#"{{"type":"shutdown"}}"#);
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct MlxAdapter {
    bridge: Mutex<BridgeProcess>,
    model_id: String,
}

impl MlxAdapter {
    pub fn load(model_path: &str, model_id: &str) -> Result<Self, String> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            return Err("MLX models are supported only on Apple Silicon macOS builds".to_string());
        }

        let path = Path::new(model_path);
        if !path.is_dir() {
            return Err(format!(
                "MLX model directory does not exist: {}",
                model_path
            ));
        }

        let python = resolve_python()?;
        let start = Instant::now();
        log::info!("MLX model load starting: id={}", model_id);

        let mut child = Command::new(&python)
            .arg("-u")
            .arg("-c")
            .arg(PYTHON_BRIDGE)
            .arg(model_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                format!(
                    "Failed to start MLX Python bridge with '{}': {}",
                    python.display(),
                    e
                )
            })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture MLX bridge stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture MLX bridge stdout".to_string())?;

        let mut bridge = BridgeProcess {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        };

        match read_bridge_message(&mut bridge.stdout)? {
            BridgeMessage::Ready => {
                log::info!(
                    "MLX model {} loaded in {}ms",
                    model_id,
                    start.elapsed().as_millis()
                );
                Ok(Self {
                    bridge: Mutex::new(bridge),
                    model_id: model_id.to_string(),
                })
            }
            BridgeMessage::LoadFailed { error } => Err(format!("MLX model load failed: {}", error)),
            other => Err(format!("Unexpected MLX bridge load message: {:?}", other)),
        }
    }
}

impl ModelAdapter for MlxAdapter {
    fn generate(
        &self,
        prompt: &PromptPackage,
        cancel: &CancellationToken,
        progress_cb: &dyn Fn(&str, u32),
    ) -> Result<GenerationResult, String> {
        if cancel.is_canceled() {
            return Err("Job canceled before MLX generation".to_string());
        }

        let start = Instant::now();
        let mut bridge = self
            .bridge
            .lock()
            .map_err(|e| format!("MLX bridge lock poisoned: {}", e))?;

        let request = json!({
            "type": "generate",
            "system_prompt": &prompt.system_prompt,
            "user_message": &prompt.user_message,
            "max_tokens": prompt.max_tokens,
            "temperature": prompt.temperature,
        });

        serde_json::to_writer(&mut bridge.stdin, &request)
            .map_err(|e| format!("Failed to encode MLX request: {}", e))?;
        bridge
            .stdin
            .write_all(b"\n")
            .map_err(|e| format!("Failed to write MLX request: {}", e))?;
        bridge
            .stdin
            .flush()
            .map_err(|e| format!("Failed to flush MLX request: {}", e))?;

        let response = {
            let BridgeProcess { stdin, stdout, .. } = &mut *bridge;
            read_bridge_message_with_cancel(stdout, stdin, cancel)?
        };

        match response {
            BridgeMessage::Completed {
                output_text,
                total_tokens,
            } => {
                if cancel.is_canceled() {
                    return Err("Job canceled during MLX generation".to_string());
                }
                progress_cb(&output_text, total_tokens);
                let duration_ms = start.elapsed().as_millis() as u64;
                log::info!(
                    "MLX generation complete: model={}, tokens={}, duration_ms={}",
                    self.model_id,
                    total_tokens,
                    duration_ms
                );
                Ok(GenerationResult {
                    output_text,
                    total_tokens,
                    duration_ms,
                })
            }
            BridgeMessage::Canceled => Err("Job canceled during MLX generation".to_string()),
            BridgeMessage::Failed { error, .. } => Err(format!("MLX generation failed: {}", error)),
            BridgeMessage::LoadFailed { error } => {
                Err(format!("MLX bridge reported load failure: {}", error))
            }
            BridgeMessage::Ready => Err("Unexpected MLX bridge ready message".to_string()),
        }
    }
}

fn read_bridge_message_with_cancel(
    stdout: &mut BufReader<ChildStdout>,
    stdin: &mut ChildStdin,
    cancel: &CancellationToken,
) -> Result<BridgeMessage, String> {
    std::thread::scope(|scope| {
        let (tx, rx) = std::sync::mpsc::channel();
        scope.spawn(move || {
            let _ = tx.send(read_bridge_message(stdout));
        });

        let mut cancel_sent = false;
        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(result) => return result,
                Err(RecvTimeoutError::Timeout) => {
                    if cancel.is_canceled() && !cancel_sent {
                        serde_json::to_writer(&mut *stdin, &json!({ "type": "cancel" }))
                            .map_err(|e| format!("Failed to encode MLX cancel: {}", e))?;
                        stdin
                            .write_all(b"\n")
                            .map_err(|e| format!("Failed to write MLX cancel: {}", e))?;
                        stdin
                            .flush()
                            .map_err(|e| format!("Failed to flush MLX cancel: {}", e))?;
                        cancel_sent = true;
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err("MLX bridge reader stopped before a response".to_string());
                }
            }
        }
    })
}

fn read_bridge_message(stdout: &mut BufReader<ChildStdout>) -> Result<BridgeMessage, String> {
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = stdout
            .read_line(&mut line)
            .map_err(|e| format!("Failed to read MLX bridge output: {}", e))?;
        if bytes == 0 {
            return Err("MLX bridge exited without a response".to_string());
        }

        match serde_json::from_str::<BridgeMessage>(&line) {
            Ok(message) => return Ok(message),
            Err(_) => {
                log::debug!("Ignoring non-protocol MLX bridge output");
            }
        }
    }
}

fn resolve_python() -> Result<PathBuf, String> {
    let candidates = python_candidates();

    for candidate in candidates {
        if candidate.components().count() > 1 && !candidate.exists() {
            continue;
        }

        let available = Command::new(&candidate)
            .arg("-c")
            .arg("import turboquant_mlx.generate")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);

        if available {
            return Ok(candidate);
        }
    }

    Err(
        "MLX Python runtime not found. Install mlx-lm and turboquant-mlx-full \
         in the ModuTone app data MLX environment, or set MODUTONE_MLX_PYTHON \
         to the Python executable in that environment."
            .to_string(),
    )
}

fn python_candidates() -> Vec<PathBuf> {
    let env_python = std::env::var_os("MODUTONE_MLX_PYTHON").map(PathBuf::from);
    let env_home = std::env::var_os("MODUTONE_MLX_HOME").map(PathBuf::from);
    let user_home = std::env::var_os("HOME").map(PathBuf::from);
    let current_dir = std::env::current_dir().ok();
    let resource_dir = std::env::current_exe().ok().and_then(|path| {
        path.parent()
            .and_then(Path::parent)
            .map(|p| p.join("Resources"))
    });

    python_candidates_from(env_python, env_home, user_home, current_dir, resource_dir)
}

fn push_unique(candidates: &mut Vec<PathBuf>, candidate: PathBuf) {
    if !candidates.contains(&candidate) {
        candidates.push(candidate);
    }
}

fn python_candidates_from(
    env_python: Option<PathBuf>,
    env_home: Option<PathBuf>,
    user_home: Option<PathBuf>,
    current_dir: Option<PathBuf>,
    resource_dir: Option<PathBuf>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(path) = env_python {
        push_unique(&mut candidates, path);
    }

    if let Some(path) = env_home {
        push_unique(&mut candidates, path.join("bin").join("python"));
        push_unique(
            &mut candidates,
            path.join(".venv").join("bin").join("python"),
        );
    }

    if let Some(resources) = resource_dir {
        push_unique(
            &mut candidates,
            resources
                .join("mlx")
                .join(".venv")
                .join("bin")
                .join("python"),
        );
    }

    if let Some(home) = user_home {
        push_unique(
            &mut candidates,
            home.join("Library")
                .join("Application Support")
                .join("com.modutone.desktop")
                .join("mlx")
                .join(".venv")
                .join("bin")
                .join("python"),
        );
        push_unique(
            &mut candidates,
            home.join("Library")
                .join("Application Support")
                .join("com.modutone.desktop")
                .join(".venv-mlx")
                .join("bin")
                .join("python"),
        );
        push_unique(
            &mut candidates,
            home.join(".modutone")
                .join("mlx")
                .join(".venv")
                .join("bin")
                .join("python"),
        );
    }

    if let Some(dir) = current_dir {
        push_unique(
            &mut candidates,
            dir.join(".venv-mlx").join("bin").join("python"),
        );
    }

    push_unique(&mut candidates, PathBuf::from("python3"));
    push_unique(&mut candidates, PathBuf::from("python"));

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_message_completed_deserializes() {
        let message: BridgeMessage =
            serde_json::from_str(r#"{"type":"completed","output_text":"done","total_tokens":1}"#)
                .unwrap();

        match message {
            BridgeMessage::Completed {
                output_text,
                total_tokens,
            } => {
                assert_eq!(output_text, "done");
                assert_eq!(total_tokens, 1);
            }
            other => panic!("unexpected message: {:?}", other),
        }
    }

    #[test]
    fn bridge_message_canceled_deserializes() {
        let message: BridgeMessage = serde_json::from_str(r#"{"type":"canceled"}"#).unwrap();
        assert!(matches!(message, BridgeMessage::Canceled));
    }

    #[test]
    fn python_candidates_include_gui_visible_app_data_runtime() {
        let candidates = python_candidates_from(
            Some(PathBuf::from("/custom/python")),
            Some(PathBuf::from("/custom/mlx")),
            Some(PathBuf::from("/Users/tester")),
            Some(PathBuf::from("/repo")),
            Some(PathBuf::from(
                "/Applications/ModuTone.app/Contents/Resources",
            )),
        );

        assert_eq!(candidates[0], PathBuf::from("/custom/python"));
        assert!(candidates.contains(&PathBuf::from(
            "/Users/tester/Library/Application Support/com.modutone.desktop/mlx/.venv/bin/python"
        )));
        assert!(candidates.contains(&PathBuf::from(
            "/Applications/ModuTone.app/Contents/Resources/mlx/.venv/bin/python"
        )));
        assert!(candidates.contains(&PathBuf::from("/repo/.venv-mlx/bin/python")));
    }
}
