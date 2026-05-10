# Privacy

ModuTone is designed around local-only processing and content ephemerality.
This document describes what data is handled and where it goes.

## Core Guarantees

### No Content Persistence

User input, generated output, prompts, and refinement instructions exist only in
process memory during an active session. Closing a tab or exiting the app
destroys that content.

There is no session recovery feature by design.

### No Automatic Content Network Activity

Inference runs locally with downloaded model files loaded by the worker
process. The app does not send writing content to remote APIs, telemetry, or
analytics services.

Model downloads are explicit user actions from Settings. When a user starts a
download, ModuTone contacts Hugging Face only to retrieve model files and writes
them to the app data models directory.

### No Telemetry

ModuTone has no analytics SDK, crash reporter, usage tracker, or phone-home
mechanism.

### Log Redaction

Logs contain operational metadata only:

- Timestamps.
- Model IDs and load times.
- Job IDs and durations.
- Error codes and subsystems.
- Process lifecycle events.

Logs must not contain:

- User input.
- Generated output.
- Refinement instructions.
- Prompt bodies.
- Model responses.

Privacy regression tests in TypeScript and Rust enforce these invariants.

## Data on Disk

The app writes only operational metadata.

| Data | Contains |
| --- | --- |
| Settings | Theme, model alias, visual style, motion preference |
| Profiles | User-created names and configuration |
| Custom tags | User-created tag labels and categories |
| Models | User-downloaded model weights and catalog metadata |
| Logs | Redacted operational metadata |

Default app data locations:

| Platform | Location |
| --- | --- |
| Windows | `%APPDATA%\com.modutone.desktop\` |
| macOS | `~/Library/Application Support/com.modutone.desktop/` |
| Linux | `$XDG_DATA_HOME/com.modutone.desktop/` |

If `XDG_DATA_HOME` is not set, Linux typically uses:

```text
~/.local/share/com.modutone.desktop/
```

## Content Lifecycle

```text
User enters text
  -> frontend session state in memory
  -> Tauri IPC command
  -> backend prompt construction in memory
  -> worker stdin message
  -> local model inference
  -> worker stdout events
  -> frontend display state in memory
  -> tab close or app exit destroys content
```

At no point in this lifecycle is writing content written to disk, logged, or
sent over the network.

## Privacy Blackout Mode

Privacy blackout requests OS-level window capture protection where meaningful
platform support exists.

Current behavior:

- Windows: capability is probed and reported when available.
- macOS: capability is probed and reported when available.
- Linux: reported as unsupported.

This feature is best effort. It is not a replacement for controlling what is
shared in a screen sharing session.

## Uninstall Behavior

The Windows uninstaller removes bundled model files from the install directory.
It then prompts before removing user data and defaults to keeping it.

User data contains settings, profiles, custom tags, and redacted logs. It does
not contain writing content or generated output.
