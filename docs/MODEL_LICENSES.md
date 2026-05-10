# Model and Dependency Licenses

ModuTone has separate licensing concerns for application source, third-party
dependencies, and model weights.

## Application Source Code

License: [PolyForm Noncommercial 1.0.0](../LICENSE)

The application source code, configuration, and documentation are available for
noncommercial use. Commercial use requires separate permission from the author.

This license does not cover model weights.

## Model Weights

Release packages may bundle quantized GGUF files derived from Alibaba Cloud's
Qwen 2.5 model family.

| Model | Quantization | License |
| --- | --- | --- |
| Qwen 2.5 3B Instruct | Q5_K_M | Apache 2.0 |
| Qwen 2.5 14B Instruct | Q5_K_M | Apache 2.0 |

Upstream model repositories:

- Qwen/Qwen2.5-3B-Instruct
- Qwen/Qwen2.5-14B-Instruct

The Qwen 2.5 models are released by Alibaba Cloud under Apache License 2.0.
Quantization to GGUF format does not change the upstream license terms.

The source repository tracks the model catalog but not the large GGUF files.
Builders must provide valid model files before creating release packages.

## Rust Dependencies

Notable Rust dependencies:

| Crate | License | Purpose |
| --- | --- | --- |
| tauri | MIT or Apache-2.0 | Desktop framework |
| tauri-plugin-shell | MIT or Apache-2.0 | Shell plugin |
| llama-cpp-2 | MIT | llama.cpp Rust bindings |
| tokio | MIT | Async runtime |
| serde | MIT or Apache-2.0 | Serialization |
| serde_json | MIT or Apache-2.0 | JSON serialization |
| log | MIT or Apache-2.0 | Logging facade |
| log4rs | MIT or Apache-2.0 | File logging |
| sysinfo | MIT | System information |
| chrono | MIT or Apache-2.0 | Date and time |
| uuid | MIT or Apache-2.0 | ID generation |

## JavaScript Dependencies

Notable JavaScript dependencies:

| Package | License | Purpose |
| --- | --- | --- |
| react | MIT | UI library |
| react-dom | MIT | React DOM renderer |
| zustand | MIT | State management |
| vite | MIT | Build tool |
| vitest | MIT | Test runner |
| typescript | Apache-2.0 | Type system |
| eslint | MIT | Linting |
| prettier | MIT | Formatting |
| @tauri-apps/api | MIT or Apache-2.0 | Tauri IPC client |
| @tauri-apps/cli | MIT or Apache-2.0 | Tauri CLI |
| @playwright/test | Apache-2.0 | E2E testing |

## Native Libraries

| Library | License | Purpose |
| --- | --- | --- |
| llama.cpp | MIT | Local model inference |

Exact dependency versions are pinned in:

- `Cargo.lock`
- `package-lock.json`

## Build Tools Not Bundled

| Tool | License | Purpose |
| --- | --- | --- |
| 7-Zip | LGPL-2.1 | SFX archive creation and extraction |

7-Zip and optional extractor binaries are required only for Windows release
packaging. They are not committed to the repository.
