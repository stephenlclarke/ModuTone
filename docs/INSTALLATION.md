# Installation

## Windows Release Package

Windows is the primary release target for ModuTone v1.0.0.

Download both release files:

- `ModuTone_1.0.0_x64-setup.exe`
- `ModuTone_1.0.0_x64-setup.7z`

Place the files in the same folder, then run:

```text
ModuTone_1.0.0_x64-setup.exe
```

The launcher extracts the companion payload, runs the NSIS installer, and
copies bundled GGUF model files into the application install directory.

## After Installation

Launch ModuTone from the Start Menu or desktop shortcut. On first launch:

1. ModuTone detects available system RAM.
2. The model selector shows model suitability.
3. Select a model to warm it.
4. Start writing.

## System Requirements

| Requirement | Details |
| --- | --- |
| OS | Windows 11 x64 verified |
| RAM for 3B model | 8 GB minimum |
| RAM for 14B model | 24 GB minimum |
| Disk space | About 6 GB for app and bundled models |

Windows 10 may work if WebView2 is available, but Windows 11 is the verified
release target.

## Uninstall

Use Windows Settings > Apps > ModuTone, or run the uninstaller from the
installation directory.

The uninstaller will:

- Remove the application and bundled model files.
- Ask whether to remove user data.
- Keep user data by default.

User data is stored under:

```text
%APPDATA%\com.modutone.desktop\
```

## macOS and Linux

macOS and Linux build paths are configured but not release-verified. Build from
source to produce local packages:

- macOS: DMG
- Linux: AppImage and deb

See [Build from Source](BUILD_FROM_SOURCE.md) for platform build steps.

On Apple Silicon, ModuTone can also use the optional MLX backend for
`manjunathshiva/gpt-oss-20b-tq3`. See
[Apple Silicon MLX Setup](APPLE_SILICON.md) for tool installation, model
download, and GUI launch setup.
