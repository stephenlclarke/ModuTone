# Roadmap

ModuTone v1.1.0 is feature-complete for its initial scope: local, private
writing refinement with in-app model downloads.

The items below are possible future work, not commitments.

## Platform Verification

- Verify macOS DMG installation on Apple Silicon and Intel hardware.
- Verify Linux AppImage and deb installation across common distributions.
- Add release-device smoke checks for bundled model discovery.

## Model Support

- Evaluate additional open-weight model families for writing refinement.
- Add an opt-in local model import flow.
- Add checksum verification for in-app model downloads.

## Writing Features

- Add export options for plain text and Markdown.
- Add an optional local-only session history mode.
- Add side-by-side or inline diff review.

Session history would require careful privacy design because ephemerality is a
core product principle.

## Performance

- Evaluate llama.cpp GPU backends such as CUDA, Metal, and Vulkan.
- Explore pre-warming a second model while another model is active.
- Add performance benchmarks for model load and generation latency.

## Developer Experience

- Expand E2E coverage beyond the current smoke test.
- Add package verification scripts for generated release artifacts.
- Add automated checks for bundled model catalog consistency.
