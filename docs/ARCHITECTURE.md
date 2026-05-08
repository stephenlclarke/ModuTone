# Architecture

ModuTone is a desktop application built on a three-process architecture. Each process has a distinct responsibility and communicates through well-defined boundaries.

## Process Model

```
┌─────────────────────┐     Tauri IPC      ┌─────────────────────┐
│                     │   (17 commands)     │                     │
│   Frontend          │ ◄────────────────►  │   Backend           │
│   React + TS        │                     │   Rust + Tauri 2    │
│   Zustand state     │                     │                     │
│                     │                     │                     │
└─────────────────────┘                     └──────────┬──────────┘
                                                       │
                                              stdin/stdout
                                              JSON Lines
                                                       │
                                            ┌──────────▼──────────┐
                                            │                     │
                                            │   Worker (sidecar)  │
                                            │   Rust + llama.cpp  │
                                            │                     │
                                            └─────────────────────┘
```

### Frontend (React + TypeScript)

**Location:** `src/`

The UI layer, rendered in a Tauri webview. Manages all user interaction and display state.

- **Components** (`src/components/`) — Editors, feedback dialogs, settings panels, model selector, profile management, tag system, shell (status bar, tabs).
- **State** (`src/state/`) — Four Zustand slices:
  - `metadata` — Cached backend state (settings, profiles, tags, models)
  - `modelLoading` — Model warm-up progress tracking
  - `runtime` — Worker status, active inference jobs
  - `session` — Tab state, input/output buffers (ephemeral, session-only)
- **IPC** (`src/ipc/`) — Typed wrappers around Tauri's `invoke()`. All backend communication goes through `commands.ts`. Types mirror Rust contracts in `types.ts`.

### Backend (Rust + Tauri 2)

**Location:** `src-tauri/`

The application core. Handles persistence, process management, and domain logic.

**Module structure:**

| Module | Purpose |
|--------|---------|
| `commands/` | Tauri command handlers (generation, models, platform, profiles, runtime, settings, tags) |
| `contracts/` | Request/response types, error types, events, shared enums |
| `domain/` | Core data models (jobs, models, profiles, settings, tags) |
| `services/diagnostics/` | Logger initialization with content redaction |
| `services/inference/` | ModelRegistry, JobCoordinator, WorkerSupervisor |
| `services/persistence/` | MetadataStore (settings, profiles, tags on disk) |
| `services/platform/` | OS-specific features (privacy blackout, launch-at-login) |
| `infrastructure/` | Platform-specific implementations (Windows, macOS, Linux) |
| `app/` | Lifecycle management, instance locking, shutdown |

**Initialization sequence:**
1. Resolve app data directory
2. Initialize file logger (non-fatal on failure)
3. Initialize MetadataStore (settings, profiles, tags)
4. Initialize ModelRegistry (discover available GGUF models)
5. Resolve worker binary path
6. Initialize WorkerSupervisor (process lifecycle)
7. Initialize JobCoordinator (inference job tracking)
8. Probe PlatformCapabilities
9. Spawn worker (non-blocking)

### Worker (Rust + llama.cpp)

**Location:** `src-worker/`

A standalone Rust binary that runs as a Tauri sidecar. Handles model loading and inference via `llama-cpp-2` bindings.

**Communication protocol:** JSON Lines over stdin/stdout.

**Inbound messages:**
- `LoadModel` — Load a GGUF file into memory
- `ExecuteJob` — Run inference (spawns a thread, streams progress)
- `CancelJob` — Set cancellation token for an active job
- `Shutdown` — Exit cleanly

**Outbound messages:**
- `Ready` — Worker initialized
- `ModelLoaded` / `ModelLoadFailed` — Load result
- `JobAck` — Job accepted
- `JobProgress` — Streaming token output
- `JobCompleted` / `JobFailed` / `JobCanceled` — Terminal states

**Threading model:**
- Dedicated stdin reader thread
- Main event loop (receives from stdin reader and inference threads)
- One inference thread per active job (with cancellation token)

## IPC Commands

The backend exposes 17 Tauri commands:

| Category | Commands |
|----------|----------|
| Settings | `settings_get`, `settings_update`, `model_alias_set`, `model_alias_clear` |
| Profiles | `profiles_list`, `profiles_create`, `profiles_update`, `profiles_delete`, `profiles_reset_to_default` |
| Tags | `tags_list`, `tags_create`, `tags_update`, `tags_delete` |
| Models | `models_list` |
| Runtime | `runtime_get_status`, `runtime_warm_model` |
| Generation | `generation_start_initial`, `generation_start_refinement`, `generation_cancel` |
| Platform | `app_set_launch_at_login`, `app_set_tray_enabled`, `app_set_privacy_blackout` |

All commands return `CommandResponse<T>` — either `{ ok: true, data: T }` or `{ ok: false, error: IpcError }`.

## State Management

Frontend state is organized into four independent Zustand slices:

- **Metadata slice** — Mirrors backend-persisted data (settings, profiles, tags, models). Fetched on app start, updated on mutations.
- **Model loading slice** — Tracks model warm-up status. Intermediate state between "model selected" and "model ready for inference".
- **Runtime slice** — Worker process state, active job tracking, streaming output buffers.
- **Session slice** — Tab management, per-tab input/output content. Entirely ephemeral — destroyed on tab close or app exit.

## Persistence

Only operational metadata is persisted to disk:

- User settings (theme, model preferences, visual style)
- Profiles (named tag + parameter configurations)
- Custom tags

Stored in the OS app data directory (`$APPDATA/com.modutone.desktop` on Windows).

**Never persisted:** User input text, generated output, refinement instructions, inference prompts.

## Security Boundaries

- Frontend has minimal Tauri capabilities (`core:default`, `shell:allow-open`)
- Worker process has no direct filesystem access beyond the model file path provided at load time
- All IPC is typed with explicit error contracts
- Log system redacts content automatically
