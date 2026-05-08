// Phase: 9
// LlamaCppAdapter — real inference via llama-cpp-2 bindings.
//
// The adapter holds a LlamaModel + LlamaBackend which are Send + Sync.
// Each generate() call creates its own LlamaContext (per-request, not shared).
//
// Privacy (P2, P8): Never log prompt text, system prompt, user message,
// or generated text. Only log: token count, duration, temperature, max_tokens, model ID.

use std::num::NonZeroU32;
use std::sync::Mutex;
use std::time::Instant;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use crate::cancellation::CancellationToken;
use crate::protocol::PromptPackage;

use super::{GenerationResult, ModelAdapter};

/// Maximum context window size (prompt + generation).
const MAX_CTX_SIZE: u32 = 4096;

/// How often (in tokens) to fire progress callbacks.
const PROGRESS_INTERVAL: u32 = 8;

/// LlamaCppAdapter holds the loaded model for thread-safe sharing.
///
/// `LlamaModel` is `Send + Sync`, so this struct satisfies the
/// `ModelAdapter: Send + Sync` trait bounds. `LlamaBackend` is also
/// `Send + Sync`.
///
/// The backend is wrapped in a Mutex because it's needed to create
/// contexts and it may not be safe to use concurrently (even though
/// it's Send + Sync, context creation may not be reentrant).
pub struct LlamaCppAdapter {
    backend: Mutex<LlamaBackend>,
    model: LlamaModel,
    model_id: String,
}

impl LlamaCppAdapter {
    /// Load a GGUF model from disk.
    ///
    /// - `model_path`: Absolute path to the .gguf file
    /// - `model_id`: Identifier for logging (never logs content)
    ///
    /// Returns the adapter ready for generate() calls.
    pub fn load(model_path: &str, model_id: &str) -> Result<Self, String> {
        let start = Instant::now();

        // Log file size for diagnostics (privacy-safe: no path logged, just model ID and size)
        let file_size_mb = std::fs::metadata(model_path)
            .map(|m| m.len() / (1024 * 1024))
            .unwrap_or(0);
        log::info!(
            "Model load starting: id={}, file_size_mb={}",
            model_id,
            file_size_mb
        );

        let backend = LlamaBackend::init()
            .map_err(|e| format!("Failed to initialize llama backend: {}", e))?;

        // CPU-only for Phase 9 (no GPU layers)
        let model_params = LlamaModelParams::default();

        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load model from '{}': {}", model_path, e))?;

        let load_ms = start.elapsed().as_millis() as u64;
        log::info!(
            "Model {} loaded in {}ms (params={})",
            model_id,
            load_ms,
            model.n_params()
        );

        Ok(Self {
            backend: Mutex::new(backend),
            model,
            model_id: model_id.to_string(),
        })
    }

    /// Format the prompt using the model's chat template.
    /// Falls back to a simple concatenation if no template is available.
    fn format_prompt(&self, prompt: &PromptPackage) -> Result<String, String> {
        // Try using the model's built-in chat template
        match self.model.chat_template(None) {
            Ok(template) => {
                use llama_cpp_2::model::LlamaChatMessage;
                let messages = vec![
                    LlamaChatMessage::new("system".to_string(), prompt.system_prompt.clone())
                        .map_err(|e| format!("Failed to create system message: {}", e))?,
                    LlamaChatMessage::new("user".to_string(), prompt.user_message.clone())
                        .map_err(|e| format!("Failed to create user message: {}", e))?,
                ];

                self.model
                    .apply_chat_template(&template, &messages, true)
                    .map_err(|e| format!("Failed to apply chat template: {}", e))
            }
            Err(_) => {
                // Fallback: simple prompt format
                Ok(format!(
                    "<|im_start|>system\n{}<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
                    prompt.system_prompt, prompt.user_message
                ))
            }
        }
    }
}

impl ModelAdapter for LlamaCppAdapter {
    fn generate(
        &self,
        prompt: &PromptPackage,
        cancel: &CancellationToken,
        progress_cb: &dyn Fn(&str, u32),
    ) -> Result<GenerationResult, String> {
        let start = Instant::now();

        // Check cancellation before starting
        if cancel.is_canceled() {
            return Err("Job canceled before generation".to_string());
        }

        // Format prompt using chat template
        let formatted = self.format_prompt(prompt)?;

        // Cancel checkpoint: after prompt formatting
        if cancel.is_canceled() {
            return Err("Job canceled during prompt formatting".to_string());
        }

        // Tokenize the formatted prompt
        let prompt_tokens = self
            .model
            .str_to_token(&formatted, AddBos::Always)
            .map_err(|e| format!("Tokenization failed: {}", e))?;

        // Cancel checkpoint: after tokenization
        if cancel.is_canceled() {
            return Err("Job canceled during tokenization".to_string());
        }

        let n_prompt = prompt_tokens.len() as u32;
        let max_gen = prompt.max_tokens;

        // Compute context size: prompt + max_tokens, capped at MAX_CTX_SIZE
        let n_ctx = (n_prompt + max_gen).min(MAX_CTX_SIZE);

        if n_prompt >= n_ctx {
            return Err(format!(
                "Prompt too long: {} tokens exceeds context size {}",
                n_prompt, n_ctx
            ));
        }

        // Log metadata only (P2: never log prompt text)
        log::debug!(
            "Generating: model={}, prompt_tokens={}, max_tokens={}, temperature={}, n_ctx={}",
            self.model_id,
            n_prompt,
            max_gen,
            prompt.temperature,
            n_ctx
        );

        // Create per-request context (not shared across threads)
        let n_batch: u32 = 512;
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(n_ctx))
            .with_n_batch(n_batch);

        let backend = self
            .backend
            .lock()
            .map_err(|e| format!("Backend lock poisoned: {}", e))?;
        let mut ctx = self
            .model
            .new_context(&backend, ctx_params)
            .map_err(|e| format!("Failed to create context: {}", e))?;
        drop(backend); // Release lock immediately

        // Cancel checkpoint: after context creation
        if cancel.is_canceled() {
            return Err("Job canceled during context creation".to_string());
        }

        // Process prompt tokens in chunks of n_batch.
        // A single LlamaBatch can hold at most n_batch tokens, so prompts
        // longer than that must be fed in multiple decode passes.
        let mut batch = LlamaBatch::new(n_batch as usize, 1);
        let mut pos = 0usize;
        while pos < prompt_tokens.len() {
            // Cancel checkpoint: each prompt decode batch (critical for large models)
            if cancel.is_canceled() {
                return Err("Job canceled during prompt processing".to_string());
            }

            batch.clear();
            let chunk_end = (pos + n_batch as usize).min(prompt_tokens.len());
            let is_last_chunk = chunk_end == prompt_tokens.len();

            for j in pos..chunk_end {
                // Only request logits for the very last token of the final chunk
                let logits = is_last_chunk && j == prompt_tokens.len() - 1;
                batch
                    .add(prompt_tokens[j], j as i32, &[0], logits)
                    .map_err(|e| format!("Batch add failed: {}", e))?;
            }

            ctx.decode(&mut batch)
                .map_err(|e| format!("Prompt decode failed: {}", e))?;

            pos = chunk_end;
        }

        // Set up sampler with temperature
        let mut sampler = if prompt.temperature <= 0.0 {
            LlamaSampler::greedy()
        } else {
            LlamaSampler::chain_simple([
                LlamaSampler::temp(prompt.temperature),
                LlamaSampler::dist(42), // Fixed seed for reproducibility
            ])
        };

        // Sampling loop
        let mut generated_tokens: Vec<llama_cpp_2::token::LlamaToken> = Vec::new();
        let mut output_pieces: Vec<String> = Vec::new();
        let mut n_decoded: u32 = 0;
        let mut decoder = encoding_rs::UTF_8.new_decoder();

        let max_generate = (n_ctx - n_prompt).min(max_gen);

        loop {
            if cancel.is_canceled() {
                log::debug!(
                    "Generation canceled after {} tokens for model={}",
                    n_decoded,
                    self.model_id
                );
                break;
            }

            if n_decoded >= max_generate {
                break;
            }

            // Sample next token
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            // Check for end-of-generation token
            if self.model.is_eog_token(token) {
                break;
            }

            generated_tokens.push(token);
            n_decoded += 1;

            // Decode token to text piece
            match self.model.token_to_piece(token, &mut decoder, false, None) {
                Ok(piece) => {
                    output_pieces.push(piece);
                }
                Err(_) => {
                    // Non-fatal: some tokens may not decode cleanly
                }
            }

            // Progress callback every PROGRESS_INTERVAL tokens
            if n_decoded.is_multiple_of(PROGRESS_INTERVAL) {
                let partial: String = output_pieces.iter().map(|s| s.as_str()).collect();
                progress_cb(&partial, n_decoded);
            }

            // Prepare batch for next token
            batch.clear();
            batch
                .add(token, (n_prompt + n_decoded - 1) as i32, &[0], true)
                .map_err(|e| format!("Batch add failed during generation: {}", e))?;

            ctx.decode(&mut batch)
                .map_err(|e| format!("Decode failed at token {}: {}", n_decoded, e))?;
        }

        let output_text: String = output_pieces.into_iter().collect();
        let duration_ms = start.elapsed().as_millis() as u64;

        // Log performance metadata only (P2: never log output text)
        log::info!(
            "Generation complete: model={}, tokens={}, duration_ms={}, tokens_per_sec={:.1}",
            self.model_id,
            n_decoded,
            duration_ms,
            if duration_ms > 0 {
                (n_decoded as f64) / (duration_ms as f64 / 1000.0)
            } else {
                0.0
            }
        );

        Ok(GenerationResult {
            output_text,
            total_tokens: n_decoded,
            duration_ms,
        })
    }
}
