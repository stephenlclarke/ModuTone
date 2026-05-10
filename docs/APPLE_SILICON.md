# Apple Silicon MLX Setup

This guide covers a clean Apple Silicon setup for building ModuTone and running
the optional `manjunathshiva/gpt-oss-20b-tq3` MLX model.

The standard GGUF path still works on Windows, macOS, and Linux. The MLX path
is macOS arm64 only because it depends on Apple's Metal-backed MLX runtime.

## What This Enables

ModuTone can discover and load the Hugging Face model directory:

```text
src-tauri/resources/models/gpt-oss-20b-tq3/
```

The model appears in Settings as:

```text
GPT-OSS 20B TurboQuant 3-bit
```

## Requirements

| Tool | Purpose |
| --- | --- |
| Apple Silicon Mac | Required for MLX and Metal acceleration |
| macOS 14 or newer | Verified local setup target |
| Xcode Command Line Tools | Native build and Python package support |
| Homebrew | Installs Node.js and Python |
| Node.js 20 or newer | Frontend build and npm scripts |
| npm | Project package installation |
| Rust stable | Tauri backend and worker sidecar |
| Clippy and rustfmt | Rust linting and formatting |
| Python 3.12 | MLX package runtime |
| `mlx-lm` | MLX model loading and generation |
| `turboquant-mlx-full` | TurboQuant support for the TQ3 model |
| `huggingface_hub` with `hf_xet` | Hugging Face `hf` download support |

The Tauri CLI is installed as a project dev dependency. Use the npm scripts in
this repository instead of installing a global Tauri CLI.

## Install System Build Tools

Install Xcode Command Line Tools:

```bash
xcode-select --install
```

Verify the active tools path:

```bash
xcode-select -p
```

Install Homebrew if it is not already installed:

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

On a standard Apple Silicon Homebrew install, make sure Homebrew is on your
path:

```bash
eval "$(/opt/homebrew/bin/brew shellenv)"
```

Install Node.js and Python:

```bash
brew install node
brew install python@3.12
```

Install Rust with `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
rustup default stable
rustup component add clippy rustfmt
```

Verify the toolchain:

```bash
node --version
npm --version
rustc --version
cargo --version
/opt/homebrew/bin/python3.12 --version
```

## Install ModuTone Dependencies

From the repository root:

```bash
npm ci
```

Build the debug worker once so the sidecar is available for development:

```bash
npm run build:sidecar:dev
```

## Create the MLX Environment

From the repository root:

```bash
/opt/homebrew/bin/python3.12 -m venv .venv-mlx
.venv-mlx/bin/python -m pip install --upgrade pip setuptools wheel
.venv-mlx/bin/python -m pip install \
  "huggingface_hub[hf_xet]" \
  "mlx-lm>=0.31.3" \
  "turboquant-mlx-full>=0.2.0"
```

Verify the runtime:

```bash
.venv-mlx/bin/python - <<'PY'
import huggingface_hub
import mlx_lm

print("mlx-lm", mlx_lm.__version__)
print("huggingface_hub", huggingface_hub.__version__)
PY
```

ModuTone automatically checks `.venv-mlx/bin/python` when launched from the
repository root. For other layouts, set:

```bash
export MODUTONE_MLX_PYTHON="$PWD/.venv-mlx/bin/python"
```

## Download GPT-OSS 20B TQ3

The installed app can download `GPT-OSS 20B TurboQuant 3-bit` from Settings
into the app data models directory. The command-line flow below is still useful
for source-tree packaging or repeatable local setup.

The model is about 10 GB on disk and is intentionally ignored by git.

From the repository root:

```bash
mkdir -p src-tauri/resources/models/gpt-oss-20b-tq3

.venv-mlx/bin/hf download \
  manjunathshiva/gpt-oss-20b-tq3 \
  --local-dir src-tauri/resources/models/gpt-oss-20b-tq3
```

Remove Hugging Face download metadata before packaging:

```bash
rm -rf src-tauri/resources/models/gpt-oss-20b-tq3/.cache
rm -rf src-tauri/resources/models/.cache
```

Expected files include:

```text
src-tauri/resources/models/gpt-oss-20b-tq3/config.json
src-tauri/resources/models/gpt-oss-20b-tq3/tokenizer.json
src-tauri/resources/models/gpt-oss-20b-tq3/model-00001-of-00002.safetensors
src-tauri/resources/models/gpt-oss-20b-tq3/model-00002-of-00002.safetensors
```

Validate the downloaded model:

```bash
npm run prepare:models
```

## Run in Development Mode

From the repository root:

```bash
export MODUTONE_MLX_PYTHON="$PWD/.venv-mlx/bin/python"
npm run dev
```

In the app:

1. Open Settings.
2. Download `GPT-OSS 20B TurboQuant 3-bit` if it is not already installed.
3. Select `GPT-OSS 20B TurboQuant 3-bit`.
4. Wait for the model to warm.
5. Generate or refine text.

## Build and Install the App

Build the normal local DMG:

```bash
npm run build
```

Build a DMG that includes downloaded model resources:

```bash
npm run package:macos
```

Install the generated Apple Silicon DMG cleanly:

```bash
osascript -e 'tell application "ModuTone" to quit' || true
rm -rf /Applications/ModuTone.app

rm -rf /tmp/modutone-dmg
mkdir -p /tmp/modutone-dmg
hdiutil attach -nobrowse -readonly \
  -mountpoint /tmp/modutone-dmg \
  target/release/bundle/dmg/ModuTone_1.0.0_aarch64.dmg
ditto /tmp/modutone-dmg/ModuTone.app /Applications/ModuTone.app
hdiutil detach /tmp/modutone-dmg
```

## Use with an Installed macOS App

For a local app installed in `/Applications`, use Settings to download the
model or put the model directory under the app data models directory:

```bash
mkdir -p "$HOME/Library/Application Support/com.modutone.desktop/models"

.venv-mlx/bin/hf download \
  manjunathshiva/gpt-oss-20b-tq3 \
  --local-dir "$HOME/Library/Application Support/com.modutone.desktop/models/gpt-oss-20b-tq3"
```

Make the MLX Python runtime visible to GUI launches:

```bash
launchctl setenv MODUTONE_MLX_PYTHON "$PWD/.venv-mlx/bin/python"
open -n /Applications/ModuTone.app
```

To clear the GUI environment variable:

```bash
launchctl unsetenv MODUTONE_MLX_PYTHON
```

## Troubleshooting

If the model does not appear, confirm the model directory contains
`config.json`, `tokenizer.json`, and at least one `.safetensors` file.

If model loading fails with a Python runtime error, confirm:

```bash
echo "$MODUTONE_MLX_PYTHON"
"$MODUTONE_MLX_PYTHON" -c "import mlx_lm"
```

If the app reports insufficient memory, close other memory-heavy applications
or use a smaller GGUF model.
