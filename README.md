[README.md](https://github.com/user-attachments/files/27538091/README.md)
# ModuTone

A privacy-first, local-only desktop writing refinement application. ModuTone runs large language models entirely on your machine — no cloud services, no data collection, no network requests. Your writing never leaves your device.

Built with [Tauri 2](https://tauri.app/) (Rust backend), React (TypeScript frontend), and [llama.cpp](https://github.com/ggerganov/llama.cpp) (local inference).

## What It Does

ModuTone helps you refine and improve your writing using local AI models. You provide input text and style guidance through composable tags and profiles, and ModuTone generates refined output — all processed locally.

- **Generate**: Transform input text according to selected style tags and profile
- **Refine**: Iteratively improve output with natural-language refinement instructions
- **Compare**: Review proposed changes before accepting or rejecting them

## Key Design Principles

- **Privacy by default** — No content persisted to disk. No content in logs. No telemetry. Air-gapped operation supported.
- **Local inference only** — All processing runs on-device via bundled Qwen 2.5 models (3B, 14B parameters).
- **Non-destructive editing** — Proposed output is always separate from accepted output until you explicitly accept it.
- **Composable style system** — Combine built-in and custom tags to express nuanced writing intent.

## Architecture

ModuTone uses a three-process architecture:

| Process      | Technology                   | Role                                                 |
| ------------ | ---------------------------- | ---------------------------------------------------- |
| **Frontend** | React + TypeScript + Zustand | UI, state management, user interaction               |
| **Backend**  | Rust + Tauri 2               | IPC routing, persistence, process supervision        |
| **Worker**   | Rust + llama.cpp             | Model loading, inference execution (sidecar process) |

The frontend communicates with the backend through Tauri's IPC command system (17 commands). The backend manages a worker sidecar process over stdin/stdout JSON Lines protocol.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full technical breakdown.

## Installation

**Windows (primary platform):**

1. Download `ModuTone-Setup.exe` and `ModuTone-Setup.7z` from the [latest release](releases/v1.0.0/)
2. Place both files in the same folder
3. Run `ModuTone-Setup.exe` and follow the installer prompts
4. Models install automatically during setup

See [docs/INSTALLATION.md](docs/INSTALLATION.md) for detailed instructions.

## System Requirements

| Requirement | Minimum                                                         |
| ----------- | --------------------------------------------------------------- |
| OS          | Windows 11 x64                                                  |
| RAM         | 8 GB (3B model) / 24 GB (14B model) |
| Disk        | ~3 GB for app + models                                          |

ModuTone auto-detects available RAM and shows which models are suitable for your system.

## Building from Source

Prerequisites: Node.js 24+, Rust (stable), Tauri CLI v2.

```bash
npm install
npm run build:sidecar
npm run build
```

See [docs/BUILD_FROM_SOURCE.md](docs/BUILD_FROM_SOURCE.md) for complete build instructions.

## Testing

398 tests across the Rust backend and TypeScript frontend:

```bash
# Frontend unit tests (232 tests via Vitest)
npm run test

# Rust unit + integration tests (166 tests)
npm run test:rust

# E2E tests (Playwright)
npm run test:e2e
```

See [docs/VALIDATION_REPORT.md](docs/VALIDATION_REPORT.md) for the full test report.

## Project Structure

```
src/                  React frontend (components, state, IPC, styles)
src-tauri/            Rust backend (commands, services, domain, infrastructure)
src-worker/           Inference worker sidecar (llama.cpp integration)
tests/                E2E and contract tests
scripts/              Build and packaging scripts
tools/sfx-stub/       Self-extracting installer stub (source)
docs/                 Project documentation
```

## Documentation

| Document                                       | Description                                                |
| ---------------------------------------------- | ---------------------------------------------------------- |
| [Architecture](docs/ARCHITECTURE.md)           | Three-process model, IPC contracts, state management       |
| [Privacy](docs/PRIVACY.md)                     | Content ephemerality, log redaction, local-only guarantees |
| [Installation](docs/INSTALLATION.md)           | Download and install instructions                          |
| [Build from Source](docs/BUILD_FROM_SOURCE.md) | Prerequisites and build steps                              |
| [Windows Release](docs/WINDOWS_RELEASE.md)     | Windows 11 x64 platform status                             |
| [Validation Report](docs/VALIDATION_REPORT.md) | Test results and static analysis                           |
| [Model Licenses](docs/MODEL_LICENSES.md)       | App, dependency, and model weight licensing                |
| [Roadmap](docs/ROADMAP.md)                     | Possible future work                                       |

## Technology Stack

**Frontend:** React 18, TypeScript 5.6, Zustand 5, Vite 6
**Backend:** Rust, Tauri 2, Tokio, Serde, log4rs
**Inference:** llama-cpp-2 (Rust bindings for llama.cpp)
**Models:** Qwen 2.5 (3B, 14B) — quantized GGUF format
**Testing:** Vitest 3, Playwright, Cargo test
**CI:** GitHub Actions (lint, test, build on Ubuntu/Windows/macOS)

## License

This project is source-available under the [PolyForm Noncommercial License 1.0.0](LICENSE). You may view, use, modify, and share the source code for any noncommercial purpose. Commercial use requires separate permission from the author.

Bundled model weights (Qwen 2.5) are licensed under Apache 2.0 by Alibaba Cloud. See [docs/MODEL_LICENSES.md](docs/MODEL_LICENSES.md) for details.
