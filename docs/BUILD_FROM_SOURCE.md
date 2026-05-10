# Build from Source

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | 24+ | Frontend build, scripts |
| npm | (bundled with Node) | Package management |
| Rust | stable | Backend and worker compilation |
| Tauri CLI | v2 | Desktop app bundling |
| Clippy | (Rust component) | Lint checks |
| rustfmt | (Rust component) | Format checks |

### Install Tauri CLI

```bash
npm install -g @tauri-apps/cli
```

### Linux-only system dependencies

```bash
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

## Clone and Install

```bash
git clone <repo-url>
cd modutone
npm install
```

## Build Steps

### 1. Build the worker sidecar

The worker is a separate Rust binary that must be compiled before the Tauri app:

```bash
# Release build
npm run build:sidecar

# Or debug build for development
npm run build:sidecar:dev
```

This compiles `src-worker/` and copies the binary to `src-tauri/binaries/` with the correct platform suffix (e.g., `modutone-worker-x86_64-pc-windows-msvc.exe`).

### 2. Build the application

```bash
# Development mode (hot reload)
npm run dev

# Production build
npm run build
```

`npm run build` runs `tauri build`, which:
1. Compiles the frontend with Vite
2. Compiles the Rust backend
3. Bundles everything into a platform-specific installer (NSIS on Windows, DMG on macOS, AppImage/deb on Linux)

The output is in `src-tauri/target/release/bundle/`.

### 3. Add model files (required for inference)

This step is only needed when building from source. End users get models automatically via the installer.

Download quantized GGUF files from HuggingFace and place them in `src-tauri/resources/models/`:

| Filename | Source |
|----------|--------|
| `qwen2.5-3b-instruct-q5_k_m.gguf` | [Qwen/Qwen2.5-3B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF) |
| `qwen2.5-14b-instruct-q5_k_m.gguf` | [Qwen/Qwen2.5-14B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-14B-Instruct-GGUF) |

Download the specific quantization variant listed (Q5_K_M or Q4_K_M) from each model's "Files and versions" tab on HuggingFace.

The model catalog is at `src-tauri/resources/models/model_catalog.json`. ModuTone discovers models by matching filenames in this catalog.

## Package with Models (Windows)

To create a distributable installer with bundled models:

```bash
# Validate model files, build, and create standalone bundle
npm run package:bundle

# Or create SFX self-extracting installer
npm run package:installer
```

The SFX installer requires:
- 7-Zip installed at `C:\Program Files\7-Zip\7z.exe`
- SFX stub built from `tools/sfx-stub/` (run `cargo build --release` in that directory)
- At install time, either 7-Zip installed on the target machine or `7za.exe`/`7z.exe` next to the launcher. To produce a launcher with an embedded extractor, place `tools/7za.exe` locally and build the stub with `cargo build --release --features embedded-7za`.

## Development Workflow

```bash
# Start development mode (frontend hot reload + Tauri window)
npm run dev

# Run frontend tests
npm run test

# Run Rust tests
npm run test:rust

# Lint and format
npm run lint
npm run format:check
cargo fmt --check --all
npm run lint:rust

# Type check
npm run typecheck
```

## Project Layout

| Directory | Contents |
|-----------|----------|
| `src/` | React frontend (TypeScript, components, state, IPC) |
| `src-tauri/` | Rust backend (Tauri app, commands, services, domain) |
| `src-worker/` | Rust inference worker (llama.cpp sidecar) |
| `tests/` | E2E (Playwright) and contract tests |
| `scripts/` | Build and packaging Node.js scripts |
| `tools/sfx-stub/` | SFX installer stub (Rust source) |
