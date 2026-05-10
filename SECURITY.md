# Security Policy

## Privacy by Design

ModuTone treats privacy as an architectural constraint.

- **No automatic network requests:** Inference runs locally with GGUF model
  files. The app does not send writing content to remote services.
- **No content persistence:** User input, generated output, prompts, and
  refinement instructions exist only in process memory.
- **Log redaction:** Logs contain operational metadata only, such as timestamps,
  model IDs, job IDs, durations, and error codes.
- **Minimal webview permissions:** The Tauri capability set is limited to
  `core:default` and `shell:allow-open`.
- **Controlled sidecar process:** The backend starts the bundled worker sidecar
  and passes it explicit model paths and job messages.
- **Session ephemerality:** Closing a tab or the application discards session
  content permanently.

## Supported Versions

| Version | Supported |
| --- | --- |
| 1.0.x | Yes |

## Reporting a Vulnerability

If you discover a security issue, report it responsibly:

1. Do not open a public GitHub issue for security vulnerabilities.
2. Email a description, reproduction steps, and relevant context.
3. Allow reasonable time for a fix before public disclosure.

## Scope

Security concerns relevant to ModuTone include:

- Content leaking to disk, logs, crash reports, or unexpected network calls
- IPC message injection or command escalation
- Worker sidecar process escape or privilege elevation
- Installer tampering or supply-chain integrity
- Incorrect reporting of platform privacy features

Out of scope:

- Server-side vulnerabilities, because there is no server component
- Authentication bypass, because there is no authentication system
