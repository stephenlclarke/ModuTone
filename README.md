# ModuTone

![ModuTone desktop app screenshot](docs/assets/modutone-app.png)

ModuTone is a privacy-first desktop writing refinement app. It runs local
language models on your machine and does not send writing content to cloud
services, telemetry, or remote APIs.

Built with [Tauri 2](https://tauri.app/), React, TypeScript, Rust,
[llama.cpp](https://github.com/ggerganov/llama.cpp), and optional
[MLX](https://github.com/ml-explore/mlx) support on Apple Silicon.

## What It Does

ModuTone helps you improve writing with local AI models. You provide text,
select style tags and a profile, and review generated output before accepting
it.

- **Generate:** Transform input text with the selected model, tags, and
  profile.
- **Refine:** Improve accepted output with natural-language instructions.
- **Compare:** Review proposed output before accepting or rejecting it.

## Key Design Principles

- **Privacy by default:** No writing content is persisted to disk, logged, or
  transmitted.
- **Local inference:** Generation runs on-device with local GGUF files or an
  Apple Silicon MLX model directory.
- **Explicit model downloads:** Settings can download cataloged model files
  without sending writing content off-device.
- **Guided Apple Silicon setup:** Settings can create the private MLX Python
  runtime needed by GPT-OSS TQ3.
- **Non-destructive editing:** Proposed output stays separate until accepted.
- **Composable style controls:** Tags and profiles express writing intent.

## Platform Status

| Platform | Status | Artifact |
| --- | --- | --- |
| Windows 11 x64 | Primary verified target | NSIS plus SFX payload |
| macOS | Build and test path configured | DMG |
| Linux | Build and test path configured | AppImage and deb |

The CI build matrix covers Ubuntu, Windows, and macOS. Windows remains the
primary release target until macOS and Linux packages receive release-device
verification.

## Architecture

ModuTone uses a three-process architecture:

| Process | Technology | Role |
| --- | --- | --- |
| Frontend | React, TypeScript, Zustand | UI and state |
| Backend | Rust, Tauri 2 | IPC, persistence, supervision |
| Worker | Rust, llama.cpp, optional MLX | Model loading and inference |

The frontend talks to the backend through typed Tauri IPC commands. The backend
manages the worker sidecar over a stdin/stdout JSON Lines protocol.

## Workflow

```mermaid
flowchart TD
    A[Writer enters text] --> B[Select profile, tags, and model]
    B --> C{Model installed?}
    C -- No --> D{Install source}
    D -- Settings download --> E[Backend downloads approved model files]
    E --> F[Write files to app data models directory]
    D -- Manual install --> G[Place GGUF file or MLX model folder]
    F --> H[Refresh model registry]
    G --> H
    H --> B
    C -- Yes --> I{MLX model?}
    I -- Yes --> J{MLX runtime installed?}
    J -- No --> K[Settings creates private Python MLX runtime]
    K --> L[Install mlx-lm, TurboQuant, and Hugging Face tooling]
    L --> M[Warm selected local model]
    J -- Yes --> M
    I -- No --> M
    M --> N[Worker sidecar loads model]
    N --> O[Ready for generation]
    O --> P[Generate or refine]
    P --> Q[React state sends Tauri IPC command]
    Q --> R[Rust backend validates request and composes prompt]
    R --> S[Worker runs llama.cpp or MLX inference locally]
    S --> T[Backend emits generation events]
    T --> U[Frontend updates accepted output or proposal]
    U --> V{Refine again?}
    V -- Yes --> P
    V -- No --> W[Accept, copy, or clear output]
```

See [Architecture](docs/ARCHITECTURE.md) for the full technical breakdown.

## Installation

Windows release packages use a two-file installer payload:

1. Download `ModuTone_1.0.0_x64-setup.exe`.
2. Download `ModuTone_1.0.0_x64-setup.7z`.
3. Place both files in the same folder.
4. Run `ModuTone_1.0.0_x64-setup.exe`.

The launcher extracts the payload, runs the NSIS installer, and copies bundled
models into the application install directory.

Models can also be downloaded from Settings after installation.

macOS and Linux packages can be built from source. See
[Installation](docs/INSTALLATION.md) for platform details.

## System Requirements

| Requirement | Minimum |
| --- | --- |
| OS | Windows 11 x64, macOS, or Linux |
| RAM | 8 GB for 3B model, 16 GB for MLX TQ3, 24 GB for 14B model |
| Disk | Varies by downloaded model, from about 2.5 GB to 10.5 GB |
| Apple Silicon MLX runtime | Python 3.14 preferred; Settings installs runtime |

ModuTone detects available RAM and labels models as recommended, caution, or
unsupported for the current system.
Python 3.13 and 3.12 remain supported fallbacks for Apple Silicon MLX setup.

## Building from Source

Prerequisites:

- Node.js 20 or newer
- Rust stable
- Platform dependencies required by Tauri
- Python 3.14 preferred for Apple Silicon GPT-OSS TQ3 runtime setup

```bash
npm ci
npm run build:sidecar
npm run build
```

Model files are required for inference and release packaging. See
[Build from Source](docs/BUILD_FROM_SOURCE.md) for model setup and packaging
commands. On Apple Silicon, see
[Apple Silicon MLX Setup](docs/APPLE_SILICON.md) to run
`manjunathshiva/gpt-oss-20b-tq3`.

## Testing

Current local validation covers 433 test cases:

```bash
# Frontend, contract, and TypeScript tests: 246 tests
npm run test

# Rust backend and worker tests: 186 tests
npm run test:rust

# Playwright smoke test: 1 test
npm run test:e2e
```

See [Validation Report](docs/VALIDATION_REPORT.md) for the full command list.

## Project Structure

```text
src/                  React frontend
src-tauri/            Rust backend and Tauri app
src-worker/           Inference worker sidecar
tests/                E2E and contract tests
scripts/              Build and packaging scripts
tools/sfx-stub/       Self-extracting installer launcher
docs/                 Project documentation
```

## Documentation

| Document | Description |
| --- | --- |
| [Architecture](docs/ARCHITECTURE.md) | Process model and IPC flow |
| [Privacy](docs/PRIVACY.md) | Content lifecycle and local-only guarantees |
| [Installation](docs/INSTALLATION.md) | Release installation steps |
| [Build from Source](docs/BUILD_FROM_SOURCE.md) | Build and packaging workflow |
| [Apple Silicon MLX Setup](docs/APPLE_SILICON.md) | GPT-OSS 20B TQ3 setup for Apple Silicon |
| [Windows Release](docs/WINDOWS_RELEASE.md) | Windows installer details |
| [Validation Report](docs/VALIDATION_REPORT.md) | Test and CI coverage |
| [Model Licenses](docs/MODEL_LICENSES.md) | Code, dependency, and model licenses |
| [Roadmap](docs/ROADMAP.md) | Possible future work |

## Technology Stack

- **Frontend:** React 18, TypeScript 5.6, Zustand 5, Vite 6
- **Backend:** Rust, Tauri 2, Tokio, Serde, log4rs
- **Inference:** llama-cpp-2 bindings for llama.cpp; optional MLX bridge on
  Apple Silicon
- **Models:** Qwen 2.5 GGUF files; optional GPT-OSS 20B TQ3 MLX model on
  Apple Silicon
- **Testing:** Vitest 3, Playwright, Cargo test
- **CI:** GitHub Actions on Ubuntu, Windows, and macOS

## License

The application source is available under the
[PolyForm Noncommercial License 1.0.0](LICENSE). You may view, use, modify, and
share the source code for noncommercial purposes. Commercial use requires
separate permission from the author.

Model weights are licensed separately by their upstream authors. See
[Model Licenses](docs/MODEL_LICENSES.md).
