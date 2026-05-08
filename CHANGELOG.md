# Changelog

All notable changes to ModuTone are documented in this file.

## [1.0.0] - 2025-05-08

Initial release. Windows 11 x64 verified.

### Features

- **Local inference** — Generate and refine text using bundled Qwen 2.5 models (3B, 14B) via llama.cpp. All processing runs on-device.
- **Privacy-first design** — No content persisted to disk. No content in logs. No telemetry or network requests.
- **Composable style tags** — Combine built-in and custom tags across categories (tone, formality, audience, domain) to guide output.
- **Writing profiles** — Save and switch between named configurations of tags, model selection, and generation parameters.
- **Non-destructive refinement** — Proposed output is separate from accepted output. Accept or reject changes explicitly.
- **Iterative refinement** — Provide natural-language instructions to refine output further.
- **Multi-tab workspace** — Work on multiple writing tasks in parallel with independent state per tab.
- **Model auto-detection** — RAM-aware model selector shows which models are suitable for the current system.
- **Streaming output** — Token-by-token output display during generation with cancel support.
- **NSIS installer** — Self-extracting Windows installer with bundled models. Models copy automatically during installation.
- **Settings persistence** — Theme preference (light/dark/system), visual style, motion preferences, and privacy blackout mode.
- **Log redaction** — Structured logging with automatic content sanitization.

### Platform Support

- **Windows 11 x64** — Verified. NSIS installer with SFX model delivery.
- **macOS** — Build configuration present (DMG). Untested.
- **Linux** — Build configuration present (AppImage, deb). Untested.

### Testing

- 232 TypeScript tests (Vitest) — unit, integration, privacy regression, concurrency
- 166 Rust tests (Cargo test) — unit, integration, privacy regression, upgrade migration
- E2E smoke tests (Playwright)
- Contract tests (IPC type verification)
- Static analysis: ESLint, Clippy (deny warnings), Prettier, rustfmt
