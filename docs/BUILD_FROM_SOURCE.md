# Build from Source

This guide covers development, clean local builds, and platform packaging for
Windows, macOS, and Linux.

## Prerequisites

| Tool | Version | Purpose |
| --- | --- | --- |
| Node.js | 20 or newer | Frontend build and scripts |
| npm | Bundled with Node | Package management |
| Rust | stable | Backend and worker builds |
| Clippy | Rust component | Rust linting |
| rustfmt | Rust component | Rust formatting |

CI currently uses Node.js 24. A clean macOS build was verified with Node.js
20.20.2, npm 10.8.2, and Rust 1.95 on Apple Silicon.

The Tauri CLI is installed as a project dev dependency. Use the npm scripts
instead of requiring a global Tauri install.

## Platform Dependencies

### macOS

Install Xcode Command Line Tools:

```bash
xcode-select --install
```

Verify they are available:

```bash
xcode-select -p
```

Install Rust with `rustup` or another stable toolchain provider, then ensure
Clippy and rustfmt are available:

```bash
rustup component add clippy rustfmt
```

Apple Silicon Macs can also run the optional MLX backend for
`manjunathshiva/gpt-oss-20b-tq3`. See
[Apple Silicon MLX Setup](APPLE_SILICON.md) for the Python, MLX, Hugging Face,
and model download steps.

### Linux

On Debian or Ubuntu based systems, install Tauri's WebKit dependencies:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev
sudo apt-get install -y librsvg2-dev patchelf
```

### Windows

Install:

- Node.js 20 or newer.
- Rust stable with the MSVC toolchain.
- Microsoft C++ Build Tools or Visual Studio with C++ desktop tools.
- WebView2 Runtime.
- 7-Zip if you need to create Windows SFX release packages.

## Clone and Install

For a reproducible checkout from the lockfile:

```bash
git clone <repo-url>
cd modutone
npm ci
```

For day-to-day dependency updates, use `npm install` intentionally and commit
any resulting lockfile change.

## Run in Development Mode

```bash
npm run dev
```

The dev command builds the debug worker sidecar, starts Vite, and launches the
Tauri app window.

## Worker Sidecar

The worker is a separate Rust binary. Build it directly when running Rust checks
or when you want to verify sidecar generation:

```bash
# Release sidecar
npm run build:sidecar

# Debug sidecar for development and tests
npm run build:sidecar:dev
```

The copy script writes the sidecar into `src-tauri/binaries/` with the current
platform suffix, such as `modutone-worker-aarch64-apple-darwin`.

`npm run build` also runs `npm run build:sidecar` through Tauri's
`beforeBuildCommand`, so a separate sidecar build is not required for a normal
production build.

## Production Build

```bash
npm run build
```

This command:

1. Builds the release worker sidecar.
2. Builds the frontend with Vite.
3. Builds the Rust Tauri app.
4. Bundles the platform artifact.

Default build artifacts do not include large GGUF model weights. They can
launch, but inference requires user-provided or bundled model files.

## Clean macOS Install

Use this sequence to rebuild and install the app cleanly on macOS:

```bash
# Optional: quit and remove a previous local install.
osascript -e 'tell application "ModuTone" to quit' || true
rm -rf /Applications/ModuTone.app

# Optional: remove generated repo artifacts.
rm -rf node_modules dist target src-tauri/binaries

# Reinstall dependencies and build.
npm ci
npm run build

# Install from the generated DMG.
rm -rf /tmp/modutone-dmg
mkdir -p /tmp/modutone-dmg
hdiutil attach -nobrowse -readonly \
  -mountpoint /tmp/modutone-dmg \
  target/release/bundle/dmg/ModuTone_1.0.0_aarch64.dmg
ditto /tmp/modutone-dmg/ModuTone.app /Applications/ModuTone.app
hdiutil detach /tmp/modutone-dmg

# Launch the installed app.
open -n /Applications/ModuTone.app
```

To reset local app metadata as part of a clean test install, remove the app data
directory before launching:

```bash
rm -rf "$HOME/Library/Application Support/com.modutone.desktop"
```

The macOS DMG build uses `target/release/bundle/macos/ModuTone.app` as a
staging bundle while creating the DMG. That staging bundle may be cleaned by the
bundler. Install from the generated DMG instead.

On Apple Silicon, the generated DMG is:

```text
target/release/bundle/dmg/ModuTone_1.0.0_aarch64.dmg
```

On Intel macOS, expect the architecture suffix to differ.

## Model Files

Model files are required for inference and release packaging. The repository
tracks `src-tauri/resources/models/model_catalog.json`, but not large model
weights.

Place valid model files in:

```text
src-tauri/resources/models/
```

Expected filenames:

| Model | Filename |
| --- | --- |
| Qwen 2.5 3B Instruct | `qwen2.5-3b-instruct-q5_k_m.gguf` |
| Qwen 2.5 14B Instruct | `qwen2.5-14b-instruct-q5_k_m-00001-of-00003.gguf` |
| Qwen 2.5 14B Instruct | `qwen2.5-14b-instruct-q5_k_m-00002-of-00003.gguf` |
| Qwen 2.5 14B Instruct | `qwen2.5-14b-instruct-q5_k_m-00003-of-00003.gguf` |
| GPT-OSS 20B TurboQuant 3-bit | `gpt-oss-20b-tq3/` |

The app can download cataloged models from Settings into the user models
directory. For source-tree packaging, download the matching Q5_K_M GGUF
variants from the upstream Qwen model pages on Hugging Face. On Apple Silicon,
download
`manjunathshiva/gpt-oss-20b-tq3` as an MLX model directory by following
[Apple Silicon MLX Setup](APPLE_SILICON.md).

The catalog checks GGUF filenames or shard sets and rejects truncated GGUF
downloads that are below the install-size threshold. MLX model directories must
contain
`config.json`, `tokenizer.json`, and at least one `.safetensors` file.

Validate local model files with:

```bash
npm run prepare:models
```

This command fails when no valid GGUF files are present. On Apple Silicon, a
valid `gpt-oss-20b-tq3/` MLX model directory also satisfies the check.

## Packaging with Models

Use these scripts only after valid model files are in place:

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
