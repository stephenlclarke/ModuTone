# Installation

Use the release package for your operating system. These packages install the
app only; model files are downloaded from Settings after first launch.

## Windows

Required release file:

- `ModuTone_1.1.0_x64-setup.exe`

Run the installer:

```powershell
Start-Process .\ModuTone_1.1.0_x64-setup.exe
```

Launch:

```powershell
Start-Process "shell:Start Menu\Programs\ModuTone.lnk"
```

Uninstall:

```powershell
Start-Process "ms-settings:appsfeatures"
```

Then select ModuTone and choose Uninstall.

Remove user data, downloaded models, logs, and runtime files:

```powershell
Remove-Item -Recurse -Force "$env:APPDATA\com.modutone.desktop" `
  -ErrorAction SilentlyContinue
```

## macOS

Required release file:

- `ModuTone_1.1.0_aarch64.dmg` for Apple Silicon, built without bundled models

Install:

```bash
rm -rf /tmp/modutone-dmg
mkdir -p /tmp/modutone-dmg
hdiutil attach -nobrowse -readonly \
  -mountpoint /tmp/modutone-dmg \
  ./ModuTone_1.1.0_aarch64.dmg
ditto /tmp/modutone-dmg/ModuTone.app /Applications/ModuTone.app
hdiutil detach /tmp/modutone-dmg
```

Unsigned tester build launch:

```bash
xattr -dr com.apple.quarantine /Applications/ModuTone.app
open -n /Applications/ModuTone.app
```

Apple Silicon GPT-OSS setup:

```bash
brew install python@3.14
```

Then open ModuTone and use Settings:

1. Install MLX Runtime.
2. Download `GPT-OSS 20B TurboQuant 3-bit`.
3. Select the model.

Uninstall:

```bash
osascript -e 'tell application "ModuTone" to quit' || true
rm -rf /Applications/ModuTone.app
```

Remove user data, downloaded models, logs, and runtime files:

```bash
rm -rf "$HOME/Library/Application Support/com.modutone.desktop"
```

## Linux

Required release file, choose one:

- `ModuTone_1.1.0_amd64.deb`
- `ModuTone_1.1.0_amd64.AppImage`

Install the deb package:

```bash
sudo apt install ./ModuTone_1.1.0_amd64.deb
```

Launch the deb install:

```bash
modutone
```

Run the AppImage:

```bash
chmod +x ./ModuTone_1.1.0_amd64.AppImage
./ModuTone_1.1.0_amd64.AppImage
```

Install the AppImage for the current user:

```bash
mkdir -p "$HOME/.local/bin"
cp ./ModuTone_1.1.0_amd64.AppImage "$HOME/.local/bin/modutone"
chmod +x "$HOME/.local/bin/modutone"
```

Launch the user AppImage install:

```bash
"$HOME/.local/bin/modutone"
```

Uninstall the deb package:

```bash
sudo apt remove modutone
```

Uninstall the user AppImage install:

```bash
rm -f "$HOME/.local/bin/modutone"
```

Remove user data, downloaded models, logs, and runtime files:

```bash
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/com.modutone.desktop"
```

## First Launch

Open Settings and download a model. Select the model after the download
completes.
