# Installation

## System Requirements

| Requirement | Details |
| --- | --- |
| Windows OS | Windows 11 x64 verified |
| macOS | Apple Silicon and Intel build paths configured |
| Linux | AppImage and deb build paths configured |
| RAM for 3B model | 8 GB minimum |
| RAM for GPT-OSS TQ3 MLX model | 16 GB minimum on Apple Silicon |
| RAM for 14B model | 24 GB minimum |
| Disk space | Varies by downloaded model, from about 2.5 GB to 10.5 GB |
| Apple Silicon MLX runtime | Python 3.12, `mlx-lm`, `turboquant-mlx-full` |

Windows is the primary release target for ModuTone v1.0.0. macOS and Linux
package paths are configured but not release-verified.

## Windows Install

Download both release files:

- `ModuTone_1.0.0_x64-setup.exe`
- `ModuTone_1.0.0_x64-setup.7z`

Place both files in the same folder, then run:

```text
ModuTone_1.0.0_x64-setup.exe
```

The launcher extracts the companion payload, runs the NSIS installer, and
copies bundled GGUF model files into the application install directory.

Windows 10 may work if WebView2 is available, but Windows 11 is the verified
release target. Windows 10 version 1803 and later, including Windows 11,
normally include the WebView2 Runtime already.

## Windows Uninstall

Use Windows Settings > Apps > ModuTone, or run the uninstaller from the
installation directory.

The uninstaller will:

- Remove the application and bundled model files.
- Ask whether to remove user data.
- Keep user data by default.

Windows user data is stored under:

```text
%APPDATA%\com.modutone.desktop\
```

## macOS Install

Build or download the macOS DMG. Local source builds write the Apple Silicon
DMG to:

```text
target/release/bundle/dmg/ModuTone_1.0.0_aarch64.dmg
```

Install from the DMG:

```bash
rm -rf /tmp/modutone-dmg
mkdir -p /tmp/modutone-dmg
hdiutil attach -nobrowse -readonly \
  -mountpoint /tmp/modutone-dmg \
  target/release/bundle/dmg/ModuTone_1.0.0_aarch64.dmg
ditto /tmp/modutone-dmg/ModuTone.app /Applications/ModuTone.app
hdiutil detach /tmp/modutone-dmg
open -n /Applications/ModuTone.app
```

On Apple Silicon, GPT-OSS TQ3 requires a Python MLX runtime in addition to the
model files. See [Apple Silicon MLX Setup](APPLE_SILICON.md) for:

- Python 3.12 installation.
- `mlx-lm`, `turboquant-mlx-full`, and Hugging Face tooling.
- The installed-app MLX venv under
  `~/Library/Application Support/com.modutone.desktop/mlx/.venv/`.
- GPT-OSS model download and validation commands.

## macOS Uninstall

Quit ModuTone and remove the app bundle:

```bash
osascript -e 'tell application "ModuTone" to quit' || true
rm -rf /Applications/ModuTone.app
```

To remove user data, downloaded models, logs, and the optional MLX Python
environment:

```bash
rm -rf "$HOME/Library/Application Support/com.modutone.desktop"
```

## Linux Install

Build or download a Linux package. Source builds can produce:

- AppImage
- deb

See [Build from Source](BUILD_FROM_SOURCE.md) for Linux build dependencies and
packaging commands.

Install a deb package:

```bash
sudo apt install ./ModuTone_1.0.0_amd64.deb
modutone
```

Run an AppImage:

```bash
chmod +x ModuTone_1.0.0_amd64.AppImage
./ModuTone_1.0.0_amd64.AppImage
```

Optional AppImage user install:

```bash
mkdir -p "$HOME/.local/bin"
cp ModuTone_1.0.0_amd64.AppImage "$HOME/.local/bin/modutone"
chmod +x "$HOME/.local/bin/modutone"
```

## Linux Uninstall

Remove a deb package:

```bash
sudo apt remove modutone
```

Remove an AppImage install:

```bash
rm -f "$HOME/.local/bin/modutone"
```

To remove user data, downloaded models, and logs:

```bash
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/com.modutone.desktop"
```

## After Installation

Launch ModuTone from the platform launcher, Start Menu, terminal, or
`/Applications`. On first launch:

1. ModuTone detects available system RAM.
2. Settings can download cataloged models into the app data models directory.
3. The model selector shows model suitability.
4. Select a model to warm it.
5. Start writing.
