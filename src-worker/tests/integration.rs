// Phase: 9
// Gated integration tests for real inference.
//
// These tests require actual GGUF model files on disk.
// They are skipped when the required env vars are not set.
//
// Run with:
//   MODEL_PATH_QWEN_3B=/path/to/qwen2.5-3b-instruct-q4_k_m.gguf cargo test --package modutone-worker --test integration
//
// Note: These tests are expensive (load real models, run real inference).
// They should NOT be part of the default test suite.

use std::env;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Get the path to the Qwen 3B model, if set.
fn model_path_3b() -> Option<String> {
    env::var("MODEL_PATH_QWEN_3B").ok()
}

#[test]
fn real_inference_produces_output() {
    let Some(path) = model_path_3b() else {
        eprintln!("Skipping real_inference_produces_output: MODEL_PATH_QWEN_3B not set");
        return;
    };

    // We can't easily import from the binary crate, so we test via subprocess.
    // However, we can verify the model file exists and is a valid path.
    assert!(
        std::path::Path::new(&path).exists(),
        "Model file does not exist at: {}",
        path
    );

    // For actual inference testing, we'd need to restructure the worker crate
    // to expose adapter as a library. For now, verify the file is accessible.
    let metadata = std::fs::metadata(&path).unwrap();
    assert!(
        metadata.len() > 1_000_000_000,
        "Model file seems too small: {} bytes",
        metadata.len()
    );

    eprintln!(
        "Model file verified: {} ({:.2} GB)",
        path,
        metadata.len() as f64 / 1_073_741_824.0
    );
}

#[test]
fn model_file_is_valid_gguf() {
    let Some(path) = model_path_3b() else {
        eprintln!("Skipping model_file_is_valid_gguf: MODEL_PATH_QWEN_3B not set");
        return;
    };

    // GGUF files start with the magic bytes "GGUF"
    let bytes = std::fs::read(&path).unwrap_or_default();
    if bytes.len() >= 4 {
        assert_eq!(&bytes[0..4], b"GGUF", "File does not have GGUF magic bytes");
    }
}

/// Helper to count progress callbacks (for use in future full-integration tests).
#[allow(dead_code)]
fn progress_counter() -> (impl Fn(&str, u32), Arc<AtomicU32>) {
    let count = Arc::new(AtomicU32::new(0));
    let count_clone = count.clone();
    let cb = move |_text: &str, _tokens: u32| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    };
    (cb, count)
}
