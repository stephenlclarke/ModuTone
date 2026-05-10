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
| Python 3.14 | Preferred runtime for MLX model loading |
| `mlx-lm` | MLX model loading and generation |
| `turboquant-mlx-full` | TurboQuant support for the TQ3 model |
| `huggingface_hub` with `hf_xet` | Hugging Face `hf` download support |

The Tauri CLI is installed as a project dev dependency. Use the npm scripts in
this repository instead of installing a global Tauri CLI.

For a normal user install, only Python 3.14 needs to be installed manually.
ModuTone Settings can create the private runtime environment and install
`mlx-lm`, `turboquant-mlx-full`, and Hugging Face tooling into app data.
Python 3.13 and 3.12 remain supported fallbacks.

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
brew install python@3.14
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
/opt/homebrew/bin/python3.14 --version
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

## Preferred Installed-App Runtime Setup

Use this flow for a normal `/Applications/ModuTone.app` install:

1. Install Python 3.14 with Homebrew:

   ```bash
   brew install python@3.14
   ```

2. Launch ModuTone:

   ```bash
   open -n /Applications/ModuTone.app
   ```

3. Open Settings and click **Install Runtime** for the Apple Silicon MLX
   runtime.
4. Download `GPT-OSS 20B TurboQuant 3-bit` from Settings.
5. Select the model and wait for warm-up to complete.

The runtime installer searches these Python bootstrap locations:

- `MODUTONE_MLX_BOOTSTRAP_PYTHON`, when set.
- `/opt/homebrew/bin/python3.14`
- `/usr/local/bin/python3.14`
- `python3.14`
- `/opt/homebrew/bin/python3.13`
- `/usr/local/bin/python3.13`
- `python3.13`
- `/opt/homebrew/bin/python3.12`
- `/usr/local/bin/python3.12`
- `python3.12`
- `python3`

The installer creates:

```text
~/Library/Application Support/com.modutone.desktop/mlx/.venv/
```

and installs:

- `huggingface_hub[hf_xet]`
- `mlx-lm>=0.31.3`
- `turboquant-mlx-full>=0.2.0`

## Create the Source-Tree MLX Environment

From the repository root:

```bash
/opt/homebrew/bin/python3.14 -m venv .venv-mlx
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

## Manual Installed-App MLX Environment

The in-app installer is the preferred user path. The manual commands below are
useful when reproducing setup outside the app or when debugging package
installation failures.

GUI apps launched from Finder or Spotlight do not inherit shell exports, so the
default installed runtime location is inside ModuTone's app data directory:

```text
~/Library/Application Support/com.modutone.desktop/mlx/.venv/bin/python
```

Create and verify that environment with:

```bash
APP_MLX_VENV="$HOME/Library/Application Support/com.modutone.desktop/mlx/.venv"
mkdir -p "$(dirname "$APP_MLX_VENV")"

/opt/homebrew/bin/python3.14 -m venv "$APP_MLX_VENV"
"$APP_MLX_VENV/bin/python" -m pip install --upgrade pip setuptools wheel
"$APP_MLX_VENV/bin/python" -m pip install \
  "huggingface_hub[hf_xet]" \
  "mlx-lm>=0.31.3" \
  "turboquant-mlx-full>=0.2.0"

"$APP_MLX_VENV/bin/python" - <<'PY'
import huggingface_hub
import mlx_lm
import turboquant_mlx.generate

print("mlx-lm", mlx_lm.__version__)
print("huggingface_hub", huggingface_hub.__version__)
print("turboquant_mlx.generate ok")
PY
```

`MODUTONE_MLX_PYTHON` remains supported as an override:

```bash
launchctl setenv MODUTONE_MLX_PYTHON "$APP_MLX_VENV/bin/python"
```

Clear the override with:

```bash
launchctl unsetenv MODUTONE_MLX_PYTHON
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
2. Install the Apple Silicon MLX runtime if the app shows it as missing.
3. Download `GPT-OSS 20B TurboQuant 3-bit` if it is not already installed.
4. Select `GPT-OSS 20B TurboQuant 3-bit`.
5. Wait for the model to warm.
6. Generate or refine text.

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
  target/release/bundle/dmg/ModuTone_1.1.0_aarch64.dmg
ditto /tmp/modutone-dmg/ModuTone.app /Applications/ModuTone.app
hdiutil detach /tmp/modutone-dmg
```

## Use with an Installed macOS App

For a local app installed in `/Applications`, use Settings to install the MLX
runtime and download the model. The manual model placement fallback is to put
the model directory under the app data models directory:

```bash
APP_MLX_VENV="$HOME/Library/Application Support/com.modutone.desktop/mlx/.venv"
mkdir -p "$HOME/Library/Application Support/com.modutone.desktop/models"

"$APP_MLX_VENV/bin/hf" download \
  manjunathshiva/gpt-oss-20b-tq3 \
  --local-dir "$HOME/Library/Application Support/com.modutone.desktop/models/gpt-oss-20b-tq3"
```

Launch the installed app:

```bash
open -n /Applications/ModuTone.app
```

## Troubleshooting

If the model does not appear, confirm the model directory contains
`config.json`, `tokenizer.json`, and at least one `.safetensors` file.

If model loading fails with a Python runtime error, confirm:

```bash
APP_MLX_VENV="$HOME/Library/Application Support/com.modutone.desktop/mlx/.venv"
"$APP_MLX_VENV/bin/python" -c "import mlx_lm; import turboquant_mlx.generate"
```

If the app reports insufficient memory, close other memory-heavy applications
or use a smaller GGUF model.
