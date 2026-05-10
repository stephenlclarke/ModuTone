// MLX adapter for Apple Silicon model directories.
//
// This backend keeps ModuTone's worker protocol stable while delegating MLX
// execution to a small Python bridge. Prompt text is sent over stdin, not as a
// process argument, so it is not exposed through process listings.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::Mutex;
use std::time::Instant;

use serde::Deserialize;
use serde_json::json;

use crate::cancellation::CancellationToken;
use crate::protocol::PromptPackage;

use super::{GenerationResult, ModelAdapter};

const PYTHON_BRIDGE: &str = r#"
import json
import re
import sys
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
    from mlx_lm import generate
    from mlx_lm.models.cache import make_prompt_cache
    from mlx_lm.sample_utils import make_logits_processors, make_sampler
    from turboquant_mlx.generate import load_turboquant

    model_path = sys.argv[1]
    model, tokenizer = load_turboquant(model_path)
    send({"type": "ready"})
except Exception as exc:
    send({"type": "load_failed", "error": str(exc)})
    sys.exit(2)

for line in sys.stdin:
    if not line.strip():
        continue
    try:
        request = json.loads(line)
        if request.get("type") == "shutdown":
            break

        prompt = format_prompt(
            tokenizer,
            request["system_prompt"],
            request["user_message"],
        )
        prompt_cache = make_prompt_cache(model)
        sampler = make_sampler(temp=float(request["temperature"]))
        output = generate(
            model,
            tokenizer,
            prompt=prompt,
            max_tokens=int(request["max_tokens"]),
            sampler=sampler,
            logits_processors=make_logits_processors(repetition_penalty=1.1),
            prompt_cache=prompt_cache,
            verbose=False,
        )
        if isinstance(output, tuple):
            output = output[0]
        text = clean_output(str(output))
        send({
            "type": "completed",
            "output_text": text,
            "total_tokens": len(text.split()),
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

        match read_bridge_message(&mut bridge.stdout)? {
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
            BridgeMessage::Failed { error, .. } => Err(format!("MLX generation failed: {}", error)),
            BridgeMessage::LoadFailed { error } => {
                Err(format!("MLX bridge reported load failure: {}", error))
            }
            BridgeMessage::Ready => Err("Unexpected MLX bridge ready message".to_string()),
        }
    }
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
    let mut candidates = Vec::new();

    if let Ok(path) = std::env::var("MODUTONE_MLX_PYTHON") {
        candidates.push(PathBuf::from(path));
    }

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join(".venv-mlx").join("bin").join("python"));
    }

    candidates.push(PathBuf::from("python3"));
    candidates.push(PathBuf::from("python"));

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
        "MLX Python runtime not found. Install mlx-lm and turboquant-mlx-full, \
         or set MODUTONE_MLX_PYTHON to the Python executable in that environment."
            .to_string(),
    )
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
}
