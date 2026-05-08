# Model and Dependency Licenses

ModuTone involves three categories of licensing: the application source code, third-party dependencies, and bundled model weights.

## Application Source Code

**License:** [PolyForm Noncommercial 1.0.0](../LICENSE)

The ModuTone application source code is source-available under the [PolyForm Noncommercial License 1.0.0](../LICENSE). You may view, use, modify, and share the source code for any noncommercial purpose. Commercial use requires separate permission from the author. This covers all code, configuration, and documentation in the repository. It does not cover bundled model weights, which are licensed separately (see below).

## Bundled Model Weights

ModuTone bundles quantized GGUF files derived from the Qwen 2.5 model family by Alibaba Cloud.

| Model | Original | Quantization | License |
|-------|----------|-------------|---------|
| Qwen 2.5 3B Instruct | [Qwen/Qwen2.5-3B-Instruct](https://huggingface.co/Qwen/Qwen2.5-3B-Instruct) | Q5_K_M | Apache 2.0 |
| Qwen 2.5 14B Instruct | [Qwen/Qwen2.5-14B-Instruct](https://huggingface.co/Qwen/Qwen2.5-14B-Instruct) | Q5_K_M | Apache 2.0 |

The Qwen 2.5 models are released by Alibaba Cloud under the **Apache License 2.0**. The full license text is available at:
https://www.apache.org/licenses/LICENSE-2.0

Quantization to GGUF format does not change the license terms. The quantized files are derivative works under the same Apache 2.0 license.

## Third-Party Dependencies

### Rust Dependencies (Notable)

| Crate | License | Purpose |
|-------|---------|---------|
| tauri | MIT OR Apache-2.0 | Desktop application framework |
| llama-cpp-2 | MIT | Rust bindings for llama.cpp |
| tokio | MIT | Async runtime |
| serde | MIT OR Apache-2.0 | Serialization |
| log4rs | MIT OR Apache-2.0 | Logging |
| sysinfo | MIT | System information |
| chrono | MIT OR Apache-2.0 | Date/time |
| uuid | MIT OR Apache-2.0 | ID generation |

### JavaScript Dependencies (Notable)

| Package | License | Purpose |
|---------|---------|---------|
| react | MIT | UI library |
| zustand | MIT | State management |
| vite | MIT | Build tool |
| vitest | MIT | Test runner |
| typescript | Apache-2.0 | Type system |
| eslint | MIT | Linter |
| prettier | MIT | Formatter |
| @tauri-apps/api | MIT OR Apache-2.0 | Tauri IPC client |
| @playwright/test | Apache-2.0 | E2E testing |

### Native Libraries

| Library | License | Purpose |
|---------|---------|---------|
| llama.cpp | MIT | LLM inference engine (compiled into worker via llama-cpp-2) |

Full dependency trees are available via `cargo tree` (Rust) and `npm ls` (JavaScript). The `Cargo.lock` and `package-lock.json` files pin exact versions for reproducibility.

## Build Tool Requirements (Not Bundled)

The following tools are required for building the Windows installer but are **not included** in the repository:

| Tool | License | Purpose |
|------|---------|---------|
| 7-Zip | LGPL-2.1 | Archive creation for SFX installer |

These must be installed separately by the builder.
