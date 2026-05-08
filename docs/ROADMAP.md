# Roadmap

ModuTone v1.0.0 is feature-complete for its intended scope: local, private writing refinement on Windows.

The following are possible future work items, not commitments. Priorities may shift based on community interest and feasibility.

## Possible Future Work

### Platform verification
- **macOS testing and verification** — Build configurations exist (DMG bundle). Needs testing on Apple Silicon and Intel Macs.
- **Linux testing and verification** — Build configurations exist (AppImage, deb). Needs testing across distributions.

### Model support
- **Additional model families** — Evaluate other open-weight models beyond Qwen 2.5 for writing refinement quality.
- **Model download from within the app** — Currently models are bundled with the installer. An in-app download mechanism could allow users to add models after installation.

### Writing features
- **Export options** — Copy-to-clipboard is available; file export (plain text, Markdown) could be added.
- **Session history** — Optional, opt-in session history with local-only storage. Would need careful privacy design since ephemerality is a core principle.
- **Diff view** — Side-by-side or inline diff between accepted and proposed output.

### Performance
- **GPU acceleration** — llama.cpp supports CUDA, Metal, and Vulkan backends. Currently uses CPU-only inference. GPU support would significantly improve generation speed.
- **Concurrent model loading** — Pre-warm a second model while the first is active.

### Developer experience
- **Automated E2E test suite expansion** — Current E2E coverage is a smoke test. Broader workflow coverage would improve confidence for cross-platform releases.
