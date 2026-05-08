# Contributing to ModuTone

Contributions are welcome. This document covers the development workflow, code standards, and how to submit changes.

## Development Setup

1. Install prerequisites:
   - Node.js 24+
   - Rust stable toolchain (with `clippy` and `rustfmt` components)
   - Tauri CLI v2: `npm install -g @tauri-apps/cli`

2. Clone and install:
   ```bash
   git clone <repo-url>
   cd modutone
   npm install
   ```

3. Run in development mode:
   ```bash
   npm run dev
   ```
   This builds the worker sidecar in debug mode, starts the Vite dev server, and launches the Tauri window.

## Code Style

### TypeScript

- ESLint and Prettier enforce style. Check before committing:
  ```bash
  npm run lint
  npm run format:check
  ```
- React components use functional components with hooks.
- State management uses Zustand slices. Keep state minimal and derived values computed.
- IPC calls go through `src/ipc/commands.ts` — never call `invoke()` directly from components.

### Rust

- `cargo fmt` and `cargo clippy` enforce style. Clippy runs with `-D warnings` (all warnings are errors):
  ```bash
  cargo fmt --check --all
  cargo clippy --workspace -- -D warnings
  ```
- Error handling uses typed `IpcError` responses, not panics.
- No user content in log messages. Use the redaction patterns established in `src-tauri/src/services/diagnostics/`.

## Testing

Run all tests before submitting a PR:

```bash
# TypeScript tests
npm run test

# Rust tests
cargo test --workspace

# Type checking
npm run typecheck

# E2E tests (requires a built app)
npm run test:e2e
```

### Test conventions

- TypeScript tests live alongside source files (`*.test.ts`, `*.test.tsx`) or in `src/tests/` for cross-cutting concerns.
- Rust integration tests are in `src-tauri/tests/` and `src-worker/tests/`.
- Privacy regression tests exist in both TypeScript and Rust — do not remove them.
- Contract tests in `tests/contract/` verify IPC type alignment between frontend and backend.

## Pull Request Guidelines

1. **One concern per PR.** Keep changes focused. A bug fix and a feature should be separate PRs.
2. **Tests required.** New features need tests. Bug fixes need a regression test.
3. **Privacy invariant.** No PR should introduce content persistence, logging of user text, or network calls.
4. **CI must pass.** The GitHub Actions pipeline runs lint, typecheck, format check, and all tests.
5. **Describe the change.** PR description should explain what changed and why.

## Architecture Notes

Before making structural changes, review [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Key boundaries:

- **Frontend** talks to **Backend** only through Tauri IPC commands (`src/ipc/`).
- **Backend** talks to **Worker** only through stdin/stdout JSON Lines (`src-tauri/src/services/inference/`).
- **Domain types** are defined in `src-tauri/src/domain/` and mirrored in `src/ipc/types.ts`.
- **State slices** are independent Zustand stores in `src/state/`.

## License

By contributing, you agree that your contributions will be licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE).
