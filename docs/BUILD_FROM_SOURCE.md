# Build from Source

## Prerequisites

| Tool | Version | Purpose |
| --- | --- | --- |
| Node.js | 24 or newer | Frontend build and scripts |
| npm | Bundled with Node | Package management |
| Rust | stable | Backend and worker builds |
| Clippy | Rust component | Rust linting |
| rustfmt | Rust component | Rust formatting |

The Tauri CLI is installed as a project dev dependency. A global Tauri CLI is
optional when using the npm scripts.

## Linux System Dependencies

On Debian or Ubuntu based systems, install Tauri's WebKit dependencies:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev
sudo apt-get install -y librsvg2-dev patchelf
```

## Clone and Install

```bash
git clone <repo-url>
cd modutone
npm install
```

## Worker Sidecar

The worker is a separate Rust binary. Build it before running Rust checks or a
Tauri build:

```bash
# Release sidecar
npm run build:sidecar

# Debug sidecar for development and tests
npm run build:sidecar:dev
```

The copy script writes the sidecar into `src-tauri/binaries/` with the current
platform suffix.

## Application Build

```bash
# Development mode
npm run dev

# Production build without bundled GGUF weights
npm run build
```

`npm run build` runs the Tauri build flow:

1. Compile the frontend with Vite.
2. Compile the Rust backend.
3. Bundle a platform artifact.

Output artifacts are written under:

```text
src-tauri/target/release/bundle/
```

## Model Files

Model files are required for inference and release packaging. The repository
tracks `src-tauri/resources/models/model_catalog.json`, but not the large GGUF
weights.

Place valid model files in:

```text
src-tauri/resources/models/
```

Expected filenames:

| Model | Filename |
| --- | --- |
| Qwen 2.5 3B Instruct | `qwen2.5-3b-instruct-q5_k_m.gguf` |
| Qwen 2.5 14B Instruct | `qwen2.5-14b-instruct-q5_k_m.gguf` |

Download the matching Q5_K_M GGUF variants from the upstream Qwen model pages
on Hugging Face. The catalog checks filenames and rejects truncated files that
are below the install-size threshold.

Validate local model files with:

```bash
npm run prepare:models
```

This command now fails when no valid GGUF files are present.

## Packaging with Models

Use these scripts after model files are in place:

```bash
# Windows folder bundle
npm run package:bundle

# Windows SFX launcher plus payload archive
npm run package:installer

# Linux package with bundled models
npm run package:linux

# macOS package with bundled models
npm run package:macos
```

The Windows SFX script requires:

- 7-Zip available at `C:\Program Files\7-Zip\7z.exe`, or `SEVEN_ZIP_PATH` set.
- The SFX stub built from `tools/sfx-stub/`.
- A companion or installed extractor at install time.

To embed the extractor in the launcher, place `tools/7za.exe` locally and build
the stub with:

```bash
cargo build --release --features embedded-7za
```

## Validation

```bash
npm run typecheck
npm run lint
npm run format:check
npm run test
cargo fmt --check --all
npm run lint:rust
npm run test:rust
npm run test:e2e
```

## Project Layout

| Directory | Contents |
| --- | --- |
| `src/` | React frontend |
| `src-tauri/` | Rust backend and Tauri app |
| `src-worker/` | Rust inference worker |
| `tests/` | E2E and contract tests |
| `scripts/` | Build and packaging scripts |
| `tools/sfx-stub/` | Rust SFX launcher source |
