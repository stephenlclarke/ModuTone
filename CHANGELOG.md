# Changelog

All notable changes to ModuTone are documented in this file.

## [Unreleased]

### Fixed

- Resolve bundled model discovery through Tauri's resource directory instead of
  hard-coded OS bundle paths.
- Report privacy blackout as supported only on platforms with meaningful window
  content protection support.
- Use platform-independent Node file APIs in the Windows SFX installer script.
- Fail release packaging when no valid GGUF model files are present.

### Changed

- Add an in-app model downloader for cataloged Qwen GGUF models and the Apple
  Silicon GPT-OSS MLX model.
- Correct the Qwen 2.5 14B Q5_K_M catalog entry to use the official sharded
  GGUF files.
- Add optional Apple Silicon MLX backend support for
  `manjunathshiva/gpt-oss-20b-tq3`.
- Add Apple Silicon setup documentation for MLX tooling and model download.
- Document current Windows, macOS, and Linux build status.
- Update installer filenames, model packaging requirements, and test counts.
- Add Rust workspace tests to each OS leg of the CI build matrix.

## [1.0.0] - 2025-05-08

Initial release. Windows 11 x64 verified.

### Features

- Local inference with bundled Qwen 2.5 GGUF models through llama.cpp.
- Privacy-first design with no content persistence, telemetry, or content logs.
- Composable style tags across tone, formality, audience, and domain.
- Writing profiles for reusable tag, model, and generation settings.
- Non-destructive refinement with explicit accept and reject controls.
- Natural-language refinement instructions.
- Multi-tab workspace with independent session state per tab.
- RAM-aware model selector.
- Streaming output with cancel support.
- NSIS installer with SFX model delivery.
- Settings persistence for appearance, motion, model, and privacy preferences.
- Structured logging with content redaction.

### Platform Support

- Windows 11 x64 verified with NSIS and SFX model delivery.
- macOS build configuration present for DMG artifacts.
- Linux build configuration present for AppImage and deb artifacts.

### Testing

- TypeScript tests through Vitest.
- Rust tests through Cargo.
- Playwright smoke test.
- IPC contract tests.
- Static analysis through ESLint, Clippy, Prettier, and rustfmt.
