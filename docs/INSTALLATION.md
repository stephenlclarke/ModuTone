# Installation

## Windows (Recommended)

### Download

Download both files from the [latest release](../releases/v1.0.0/):

- `ModuTone-Setup.exe` — Self-extracting installer launcher
- `ModuTone-Setup.7z` — Companion archive containing the NSIS installer and model files

### Install

1. Place `ModuTone-Setup.exe` and `ModuTone-Setup.7z` in the **same folder**
2. Run `ModuTone-Setup.exe` and follow the installer prompts
3. Models install automatically during setup — no manual download or configuration needed

### After Installation

Launch ModuTone from the Start Menu or desktop shortcut. On first launch:

1. ModuTone detects available system RAM
2. The model selector shows which models are suitable for your system
3. Select a model — it loads automatically
4. Start writing

### System Requirements

| Requirement | Details |
|------------|---------|
| OS | Windows 11 x64 (Windows 10 may work but is untested) |
| RAM (3B model) | 8 GB minimum |
| RAM (14B model) | 24 GB minimum |
| Disk space | ~3 GB (application + all bundled models) |

### Uninstall

Use Windows Settings > Apps > ModuTone, or run the uninstaller from the installation directory.

The uninstaller will:
- Remove the application and bundled model files
- Ask whether to remove user data (settings, profiles, custom tags) — defaults to **keeping** user data

User data location: `%APPDATA%\com.modutone.desktop\`

## macOS / Linux

Build configurations exist for macOS (DMG) and Linux (AppImage, deb) but these platforms are untested. See [BUILD_FROM_SOURCE.md](BUILD_FROM_SOURCE.md) for instructions on building from source on these platforms.
