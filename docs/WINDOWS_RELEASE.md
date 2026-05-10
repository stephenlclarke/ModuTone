# Windows Release

ModuTone v1.0.0 targets Windows 11 x64 as the primary verified platform.

## Platform Status

| Platform | Status | Artifact |
| --- | --- | --- |
| Windows 11 x64 | Verified release target | NSIS plus SFX payload |
| macOS | Build and test path configured | DMG |
| Linux | Build and test path configured | AppImage and deb |

## Windows Build Details

- Installer: NSIS through Tauri's bundler.
- Model delivery: 7z payload handled by the Rust SFX launcher.
- Architecture: x86_64.
- Minimum expected OS: Windows 10 with WebView2.
- Verified OS: Windows 11.

## Release Files

The current model payload exceeds the Windows PE single-file size limit, so the
release uses an external payload pair:

- `ModuTone_1.0.0_x64-setup.exe`
- `ModuTone_1.0.0_x64-setup.7z`

Both files must stay in the same folder. The user runs the `.exe`.

## Installer Flow

1. The launcher finds the companion `.7z` payload.
2. It extracts the payload with an embedded, companion, or installed 7-Zip.
3. The NSIS installer runs.
4. The NSIS post-install hook copies `models/*.gguf` into `$INSTDIR\models`.
5. The app starts with bundled models available.

The SFX launcher source is in `tools/sfx-stub/`.

## NSIS Hooks

Custom hooks live in `src-tauri/nsis/hooks.nsh`.

Post-install:

- Copy GGUF files from `$EXEDIR\models\` to `$INSTDIR\models\`.

Pre-uninstall:

- Remove bundled model files from `$INSTDIR\models`.
- Prompt before deleting user data.
- Keep user data by default.

## App Data Location

```text
%APPDATA%\com.modutone.desktop\
```

This directory contains settings, profiles, custom tags, and redacted logs. It
does not contain writing content or generated output.

## Bundled Models

| Model | Quantization | Catalog size |
| --- | --- | --- |
| Qwen 2.5 3B Instruct | Q5_K_M | 2.44 GB |
| Qwen 2.5 14B Instruct | Q5_K_M sharded GGUF | 10.51 GB |

ModuTone detects system RAM and labels each model as recommended, caution, or
unsupported.

## Build Dependencies

The Windows packaging workflow requires tools that are not committed to the
repository:

| Tool | Purpose |
| --- | --- |
| 7-Zip | Archive creation and optional install-time extraction |
| SFX launcher binary | Built from `tools/sfx-stub/` |
| Optional `7za.exe` | Embedded extractor build input |

The default launcher can find 7-Zip through `MODUTONE_7ZA_PATH`,
`SEVEN_ZIP_PATH`, a companion executable, common install paths, or `PATH`.
