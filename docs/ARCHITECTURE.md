# Architecture

ModuTone is a desktop app with three cooperating processes. Each process owns a
clear responsibility and communicates through typed boundaries.

## Process Model

```text
Frontend webview  <---- Tauri IPC ---->  Rust backend
React + Zustand                         Tauri commands
                                           |
                                           | stdin/stdout JSON Lines
                                           v
                                      Worker sidecar
                                      Rust + llama.cpp
```

## Frontend

Location: `src/`

The frontend runs inside the Tauri webview.

Main responsibilities:

- Render editors, model controls, settings, feedback, tabs, and status.
- Keep session content in memory-only Zustand state.
- Call typed IPC wrappers in `src/ipc/commands.ts`.
- Listen for backend generation and runtime events.

State slices:

- `metadata`: settings, profiles, tags, models, and RAM metadata.
- `modelLoading`: warm-up progress for the selected model.
- `runtime`: worker state, loaded model, and active job state.
- `session`: per-tab input, accepted output, proposals, and UI state.

## Backend

Location: `src-tauri/`

The backend is the Tauri application core. It handles persistence, IPC command
validation, model discovery, platform capability checks, and worker lifecycle.

| Module | Purpose |
| --- | --- |
| `commands/` | Tauri command handlers |
| `contracts/` | IPC requests, responses, events, and errors |
| `domain/` | Core settings, model, job, profile, and tag types |
| `services/diagnostics/` | Log setup and redaction |
| `services/inference/` | Model registry, jobs, worker supervision |
| `services/persistence/` | Settings, profiles, tags, and migrations |
| `services/platform/` | Privacy blackout and other platform probes |
| `infrastructure/` | OS-specific implementation boundary |
| `app/` | Lifecycle, instance lock, and shutdown helpers |

Initialization sequence:

1. Resolve the app data directory.
2. Initialize file logging.
3. Initialize `MetadataStore`.
4. Resolve Tauri's resource directory.
5. Initialize `ModelRegistry`.
6. Resolve the worker sidecar path.
7. Initialize `WorkerSupervisor`.
8. Initialize `JobCoordinator`.
9. Probe `PlatformCapabilities`.
10. Spawn the worker in the background.

## Model Discovery

The model registry discovers GGUF files and Apple Silicon MLX model directories
from two locations:

1. Bundled models under Tauri's resource directory at `models/`.
2. User models under the app data directory at `models/`.

Environment overrides:

- `MODUTONE_BUNDLED_MODELS_DIR`
- `MODUTONE_USER_MODELS_DIR`

In debug builds, the source-tree model directory is preferred:

```text
src-tauri/resources/models/
```

Release builds use Tauri's resource resolver so Windows, macOS, Linux deb, and
Linux AppImage layouts stay platform independent.

The optional MLX backend is macOS arm64 only and is documented in
[Apple Silicon MLX Setup](APPLE_SILICON.md).

## Worker

Location: `src-worker/`

The worker is a Rust sidecar that owns model loading and inference. GGUF models
use llama.cpp. Apple Silicon MLX models use a Python bridge through the same
worker protocol.

Inbound messages:

- `LoadModel`
- `ExecuteJob`
- `CancelJob`
- `Shutdown`

Outbound messages:

- `Ready`
- `ModelLoaded`
- `ModelLoadFailed`
- `JobAck`
- `JobProgress`
- `JobCompleted`
- `JobFailed`
- `JobCanceled`

The worker reads stdin on a dedicated thread. Inference jobs run on worker
threads with cancellation tokens.

## IPC Commands

The backend exposes 17 Tauri commands.

| Category | Commands |
| --- | --- |
| Settings | `settings_get`, `settings_update` |
| Settings | `model_alias_set`, `model_alias_clear` |
| Profiles | `profiles_list`, `profiles_create`, `profiles_update` |
| Profiles | `profiles_delete`, `profiles_reset_to_default` |
| Tags | `tags_list`, `tags_create`, `tags_update`, `tags_delete` |
| Models | `models_list` |
| Runtime | `runtime_get_status`, `runtime_warm_model` |
| Generation | `generation_start_initial`, `generation_start_refinement` |
| Generation | `generation_cancel` |
| Platform | `app_set_launch_at_login`, `app_set_tray_enabled` |
| Platform | `app_set_privacy_blackout` |

All command wrappers return a `CommandResponse<T>` shape:

- `{ ok: true, data: T }`
- `{ ok: false, error: IpcError }`

## Persistence

Only operational metadata is written to disk:

- User settings.
- Profiles.
- Custom tags.
- Redacted logs.

Writing content, generated output, refinement instructions, and prompts are not
persisted.

## Platform Capabilities

Platform capabilities are probed at startup and exposed through runtime status.

Privacy blackout is supported only when ModuTone can rely on meaningful OS
window protection. Current support is enabled for Windows and macOS probes and
reported as unsupported on Linux.

Launch-at-login and tray toggles currently return unsupported stubs.

## Security Boundaries

- Webview permissions are limited to `core:default` and `shell:allow-open`.
- Frontend code cannot call Tauri `invoke()` directly from components.
- Worker communication is limited to typed JSON Lines messages.
- Logs must not contain user content.
- The worker only receives explicit model paths from the backend.
