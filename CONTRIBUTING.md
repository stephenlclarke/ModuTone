# Contributing to ModuTone

Contributions are welcome. This document covers local development, validation,
and pull request expectations.

## Development Setup

1. Install prerequisites:

   - Node.js 24 or newer
   - Rust stable with `clippy` and `rustfmt`
   - Platform dependencies required by Tauri

2. Clone and install dependencies:

   ```bash
   git clone <repo-url>
   cd modutone
   npm install
   ```

3. Run in development mode:

   ```bash
   npm run dev
   ```

The dev command builds the worker sidecar, starts Vite, and launches the Tauri
window.

## Code Style

### TypeScript

- ESLint and Prettier enforce frontend style.
- React components use functional components with hooks.
- State management uses Zustand slices.
- IPC calls go through `src/ipc/commands.ts`.

Run these checks before committing frontend changes:

```bash
npm run lint
npm run format:check
npm run typecheck
```

### Rust

- `cargo fmt` and `cargo clippy` enforce Rust style.
- Clippy runs with `-D warnings`.
- IPC errors should use typed `IpcError` responses.
- Do not log user input, generated output, prompts, or refinements.

Run these checks before committing Rust changes:

```bash
cargo fmt --check --all
npm run lint:rust
```

Use `npm run lint:rust` rather than calling Clippy directly when possible. It
builds and copies the worker sidecar before the workspace lint.

## Testing

Run the relevant tests before submitting a pull request:

```bash
npm run test
npm run test:rust
npm run test:e2e
```

Test conventions:

- TypeScript tests live beside source files or in `src/tests/`.
- Rust integration tests live in `src-tauri/tests/` and `src-worker/tests/`.
- Contract tests in `tests/contract/` verify IPC type alignment.
- Privacy regression tests exist in TypeScript and Rust.

Do not remove privacy or contract coverage when changing related behavior.

## Documentation

Update documentation when behavior, packaging, platform support, security
posture, or developer workflow changes.

Markdown should pass:

```bash
markdownlint README.md CONTRIBUTING.md SECURITY.md CHANGELOG.md docs/*.md
```

## Pull Request Guidelines

1. Keep each pull request focused on one concern.
2. Add tests for new features and regression tests for bug fixes.
3. Preserve the privacy invariant: no content persistence, content logging, or
   unexpected network activity.
4. Ensure CI passes. The workflow runs linting, type checking, formatting,
   frontend tests, Rust tests, and builds on Ubuntu, Windows, and macOS.
5. Use Conventional Commits for commit messages and pull request titles.
6. Explain what changed and why in the pull request description.

## Architecture Notes

Before making structural changes, review [Architecture](docs/ARCHITECTURE.md).

Key boundaries:

- Frontend code talks to the backend only through Tauri IPC wrappers.
- Backend code talks to the worker through stdin/stdout JSON Lines.
- Rust contract types live in `src-tauri/src/contracts/`.
- TypeScript IPC types live in `src/ipc/types.ts`.
- Session content belongs only in ephemeral frontend state.

## License

By contributing, you agree that your contributions use the
[PolyForm Noncommercial License 1.0.0](LICENSE).
