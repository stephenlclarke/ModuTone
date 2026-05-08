# ModuTone v1.0.0

Initial release. Windows 11 x64 verified.

## Release Artifacts

| File | Description |
|------|-------------|
| `ModuTone-Setup.exe` | Self-extracting installer launcher |
| `ModuTone-Setup.7z` | Companion archive (NSIS installer + model files) |

Both files must be placed in the **same folder** before running the `.exe`.

## Bundled Models

| Model | Parameters | Quantization | Size | Minimum RAM |
|-------|-----------|-------------|------|-------------|
| Qwen 2.5 3B Instruct | 3B | Q5_K_M | ~2.3 GB | 8 GB |
| Qwen 2.5 14B Instruct | 14B | Q5_K_M | ~2.6 GB | 24 GB |

Both models are included in the installer. ModuTone detects available system RAM and shows which models are suitable.

Model weights are licensed under Apache 2.0 (Alibaba Cloud / Qwen team). See [docs/MODEL_LICENSES.md](../../docs/MODEL_LICENSES.md).

## Installation

1. Download both `ModuTone-Setup.exe` and `ModuTone-Setup.7z`
2. Place them in the same folder
3. Run `ModuTone-Setup.exe` and follow the installer prompts
4. Models install automatically during setup

See [docs/INSTALLATION.md](../../docs/INSTALLATION.md) for detailed instructions.

## System Requirements

- Windows 11 x64 (Windows 10 may work but is untested)
- 8 GB RAM minimum (for 3B model)
- ~3 GB disk space (application + all models)

## What's Included

- Local-only text refinement using on-device LLM inference
- Composable style tag system with built-in and custom tags
- Writing profiles for saving tag and parameter configurations
- Multi-tab workspace with independent state per tab
- Streaming token output with cancel support
- Privacy-first: no content persistence, no telemetry, no network requests
- Settings: theme (light/dark/system), visual style, motion preferences, privacy blackout mode

## Checksums

Checksums for release artifacts will be listed here once the release build is finalized.
