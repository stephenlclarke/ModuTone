# Security Policy

## Privacy by Design

ModuTone is built around privacy as a core architectural constraint, not an afterthought:

- **No network requests** — The application makes zero outbound connections. All inference runs locally via bundled model weights.
- **No content persistence** — User input and generated output exist only in process memory during the session. Nothing is written to disk.
- **Log redaction** — The structured logging system automatically sanitizes any content that could leak user text. Logs contain only operational metadata (timestamps, model IDs, job durations).
- **Minimal permissions** — The Tauri capability set is restricted to `core:default` and `shell:allow-open` (for opening external links). No filesystem write access, no clipboard monitoring, no background processes.
- **Session ephemerality** — Closing a tab or the application discards all associated content permanently. There is no undo/recovery mechanism by design.

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |

## Reporting a Vulnerability

If you discover a security issue, please report it responsibly:

1. **Do not** open a public GitHub issue for security vulnerabilities.
2. Email a description of the vulnerability, steps to reproduce, and any relevant context.
3. Allow reasonable time for a fix before public disclosure.

## Scope

Security concerns relevant to ModuTone include:

- Content leaking to disk, logs, or crash reports
- IPC message injection or command escalation
- Sidecar process escape or privilege elevation
- Installer tampering or supply-chain integrity

Out of scope (by design, not applicable):

- Network-based attacks (no network activity)
- Server-side vulnerabilities (no server component)
- Authentication bypass (no authentication system)
