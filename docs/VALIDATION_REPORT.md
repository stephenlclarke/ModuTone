# Validation Report

Validation results for the current `repo-review-fixes` branch.

Last local validation: 2026-05-10.

## Test Summary

| Category | Framework | Count | Status |
| --- | --- | --- | --- |
| TypeScript, frontend, and contract | Vitest | 246 | Pass |
| Rust backend and worker | Cargo test | 186 | Pass |
| E2E smoke | Playwright | 1 | Pass |
| Total | Mixed | 433 | Pass |

## TypeScript Tests

`npm run test` runs 246 Vitest tests across:

- Frontend component behavior.
- Zustand state slices.
- Session lifecycle and tab state transitions.
- Model loading and runtime state.
- IPC contract checks.
- Privacy regression coverage.
- Concurrency behavior.

Test files:

- `tests/contract/ipc.spec.ts`
- `src/app/ThemeProvider.test.tsx`
- `src/components/tags/TagConflictHint.test.ts`
- `src/state/metadata/concurrency.test.ts`
- `src/state/metadata/metadataSlice.test.ts`
- `src/state/modelLoading/__tests__/modelLoadingSlice.test.ts`
- `src/state/runtime/runtimeSlice.test.ts`
- `src/state/session/concurrency.test.ts`
- `src/state/session/privacy.test.ts`
- `src/state/session/sessionSlice.test.ts`
- `src/state/session/tabStateMachine.test.ts`
- `src/tests/concurrency-ipc.test.ts`
- `src/tests/privacy-regression.test.ts`

## Rust Tests

`npm run test:rust` builds the worker sidecar and runs 186 Rust tests across:

- Command validation.
- Model catalog discovery.
- Apple Silicon MLX backend protocol handling.
- Worker supervision.
- Job coordination.
- Prompt composition.
- Metadata persistence and migrations.
- Privacy regression checks.
- Worker adapter integration.

Test locations:

- Inline `#[cfg(test)]` modules in `src-tauri/src/`.
- `src-tauri/tests/privacy_regression.rs`
- `src-tauri/tests/upgrade_migration.rs`
- `src-worker/tests/integration.rs`

## E2E Tests

`npm run test:e2e` runs the Playwright smoke test in
`tests/e2e/smoke.spec.ts`.

The smoke test verifies that the app shell loads, the main editors render, the
Generate button starts disabled with no model, and a second workspace tab can be
created.

## Static Analysis

| Tool | Scope | Command |
| --- | --- | --- |
| TypeScript | Type checking | `npm run typecheck` |
| ESLint 9 | Frontend linting | `npm run lint` |
| Prettier 3 | Frontend formatting | `npm run format:check` |
| rustfmt | Rust formatting | `cargo fmt --check --all` |
| Clippy | Rust linting | `npm run lint:rust` |
| markdownlint | Markdown linting | `markdownlint ...` |

## CI Pipeline

GitHub Actions runs on pushes and pull requests targeting `main`.

The Ubuntu lint-and-test job runs:

- TypeScript type checking.
- ESLint.
- Prettier check.
- Vitest.
- Rust formatting.
- Worker sidecar preparation.
- Clippy with warnings denied.
- Cargo workspace tests.

The build matrix runs on Ubuntu, Windows, and macOS. Each matrix leg runs:

- Dependency installation.
- Worker sidecar preparation.
- Cargo workspace tests on that OS.
- Full `npm run build`.

This means platform-specific Rust behavior is tested on each supported CI
operating system before the platform build.
