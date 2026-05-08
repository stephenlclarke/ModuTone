// Phase: 9
// ModelAdapter trait — abstraction layer for inference backends.
//
// The trait requires Send + Sync because the loaded model is shared
// (via Arc) between the main thread and spawned inference threads.
// Each generate() call creates its own per-request context internally;
// only the heavy model object is shared.

pub mod llama_cpp;

use crate::cancellation::CancellationToken;
use crate::protocol::PromptPackage;

/// Thread-safe model adapter trait for inference backends.
///
/// Implementations must be Send + Sync because the adapter is held as
/// `Arc<dyn ModelAdapter>` and shared with inference threads.
pub trait ModelAdapter: Send + Sync {
    /// Run inference on the given prompt package.
    ///
    /// - `prompt`: The prompt to process (system prompt + user message + params)
    /// - `cancel`: Cooperative cancellation token checked during generation
    /// - `progress_cb`: Called periodically with (partial_text, token_count)
    ///
    /// Privacy: implementations must never log prompt text or generated output.
    fn generate(
        &self,
        prompt: &PromptPackage,
        cancel: &CancellationToken,
        progress_cb: &dyn Fn(&str, u32),
    ) -> Result<GenerationResult, String>;
}

/// Result of a completed generation.
pub struct GenerationResult {
    pub output_text: String,
    pub total_tokens: u32,
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn generation_result_construction() {
        let result = GenerationResult {
            output_text: "hello".to_string(),
            total_tokens: 5,
            duration_ms: 100,
        };
        assert_eq!(result.output_text, "hello");
        assert_eq!(result.total_tokens, 5);
        assert_eq!(result.duration_ms, 100);
    }

    /// Mock adapter for testing trait object dispatch.
    struct MockAdapter {
        response: String,
    }

    impl ModelAdapter for MockAdapter {
        fn generate(
            &self,
            _prompt: &PromptPackage,
            _cancel: &CancellationToken,
            _progress_cb: &dyn Fn(&str, u32),
        ) -> Result<GenerationResult, String> {
            Ok(GenerationResult {
                output_text: self.response.clone(),
                total_tokens: 1,
                duration_ms: 0,
            })
        }
    }

    #[test]
    fn trait_object_dispatch_via_arc() {
        let adapter: Arc<dyn ModelAdapter> = Arc::new(MockAdapter {
            response: "mocked output".to_string(),
        });
        let prompt = PromptPackage {
            system_prompt: String::new(),
            user_message: String::new(),
            max_tokens: 10,
            temperature: 0.7,
        };
        let cancel = CancellationToken::new();
        let result = adapter
            .generate(&prompt, &cancel, &|_text, _count| {})
            .unwrap();
        assert_eq!(result.output_text, "mocked output");
        assert_eq!(result.total_tokens, 1);
    }

    #[test]
    fn mock_adapter_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockAdapter>();
    }

    #[test]
    fn arc_dyn_model_adapter_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Arc<dyn ModelAdapter>>();
    }
}
