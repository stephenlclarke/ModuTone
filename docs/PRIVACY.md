# Privacy

ModuTone is designed around privacy as an architectural constraint. This document describes what data is handled, where it goes, and what guarantees the system provides.

## Core Guarantees

### No content persistence

User input, generated output, and refinement instructions exist only in process memory during an active session. When a tab is closed or the application exits, this content is gone permanently. There is no undo, no recovery, no "recently used" feature. This is by design.

### No network activity

ModuTone makes zero outbound network requests. All inference runs locally via bundled model weights loaded from disk into the worker process. The application functions identically with no network connection (air-gapped operation).

### No telemetry

There are no analytics, crash reporters, usage trackers, or phone-home mechanisms. No data about usage patterns, feature adoption, or errors is collected or transmitted.

### Log redaction

The structured logging system (log4rs) automatically redacts content. Log messages contain only operational metadata:

- Timestamps
- Model IDs and load times
- Job IDs and durations
- Error codes and subsystems
- Process lifecycle events

Log messages never contain: user input text, generated output, refinement instructions, prompt templates, or model responses.

Privacy regression tests in both TypeScript and Rust verify these invariants.

## Data on Disk

The only data ModuTone writes to disk:

| Data | Location | Contains |
|------|----------|----------|
| Settings | `$APPDATA/com.modutone.desktop` | Theme preference, model alias, visual style, motion preference |
| Profiles | `$APPDATA/com.modutone.desktop` | Named tag + parameter configurations (user-created labels, not content) |
| Custom tags | `$APPDATA/com.modutone.desktop` | Tag names and categories (user-created labels, not content) |
| Application logs | `$APPDATA/com.modutone.desktop/logs` | Operational metadata only (redacted) |

None of these contain user-authored text or model-generated content.

## Content Lifecycle

```
User types input
    ↓
Stored in Zustand session slice (memory only)
    ↓
Sent to backend via IPC command
    ↓
Backend constructs prompt, sends to worker via stdin
    ↓
Worker loads model, runs inference, streams output via stdout
    ↓
Backend relays tokens to frontend via events
    ↓
Frontend displays in UI (memory only)
    ↓
User closes tab or app → content destroyed
```

At no point in this pipeline is content written to disk, logged, or transmitted over a network.

## Privacy Blackout Mode

ModuTone supports a privacy blackout mode that requests the OS to exclude the application window from screen capture, screen sharing, and screenshots. This uses platform-native APIs where available (e.g., `SetWindowDisplayAffinity` on Windows).

## Uninstall Behavior

The NSIS uninstaller prompts the user: "Do you want to remove your ModuTone user data?" The default is No, preserving settings and profiles. If the user selects Yes, the app data directory is removed.

Bundled model files in the installation directory are always removed during uninstall.
