# Windows Release

ModuTone v1.0.0 targets Windows 11 x64 as the primary verified platform.

## Platform Status

| Platform | Status | Installer |
|----------|--------|-----------|
| Windows 11 x64 | Verified | NSIS + SFX self-extracting archive |
| macOS | Build configs present | DMG (untested) |
| Linux | Build configs present | AppImage, deb (untested) |

## Windows Build Details

- **Installer:** NSIS (Nullsoft Scriptable Install System) via Tauri's built-in bundler
- **Model delivery:** Self-extracting 7z archive (SFX stub) wrapping the NSIS installer and GGUF model files
- **Architecture:** x86_64 only
- **Minimum OS:** Windows 10 (Tauri 2 WebView2 requirement); tested on Windows 11

## Installer Flow

1. User downloads `ModuTone-Setup.exe` and `ModuTone-Setup.7z`
2. Running the `.exe` extracts the companion `.7z` archive
3. The NSIS installer runs, installing the application
4. Post-install hook copies model files from the extraction directory to the installation directory
5. Application is ready — models are available immediately

The SFX stub source code is included in `tools/sfx-stub/` for reproducibility.

## NSIS Hooks

Custom NSIS hooks (`src-tauri/nsis/hooks.nsh`) handle:

- **Post-install:** Copy `*.gguf` model files from `$EXEDIR\models\` to `$INSTDIR\models\`
- **Pre-uninstall:** Remove bundled models. Prompt user about app data removal (defaults to keeping user data).

## App Data Location

```
%APPDATA%\com.modutone.desktop\
```

Contains settings, profiles, custom tags, and logs. No user content.

## Bundled Models

| Model | Quantization | Size |
|-------|-------------|------|
| Qwen 2.5 3B Instruct | Q5_K_M | ~2.3 GB |
| Qwen 2.5 14B Instruct | Q5_K_M | ~2.6 GB |

The installer includes both models. ModuTone auto-detects system RAM and indicates which models are suitable.

## External Dependencies

The Windows build requires these tools (not included in the repository):

- **7-Zip** (`C:\Program Files\7-Zip\7z.exe`) — Used by `create-sfx-installer.js` to create the SFX archive
- **SFX stub binary** — Built from `tools/sfx-stub/` source
