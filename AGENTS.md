# AGENTS.md — Filament Material LSP

## Project Overview

VS Code extension providing LSP support for Google Filament `.mat` material files.

**Architecture**
- `client/` — VS Code extension entrypoint (`client/src/extension.ts` → `client/out/extension.js`). Downloads/caches Rust binary on first launch.
- `native/` — Rust LSP server (`filament-mat-lsp`). Full LSP implementation using `lsp-server` crate. Exposes stdio interface.
- `syntaxes/` — TextMate grammar for `.mat` files
- `test/` — Sample `.mat` files (manual test fixtures)

**Key change**: LSP logic is now fully in Rust. TypeScript client is a thin wrapper that spawns the Rust binary and communicates over stdio.

## Build & Dev Commands

```bash
# Install deps (root + client)
npm install

# Build everything (Rust release + TypeScript)
npm run compile:rust  # builds native/target/release/filament-mat-lsp
npm run compile:ts    # compiles client/src → client/out

# Or build all at once
npm run vscode:prepublish

# Watch TypeScript
npm run watch
```

**Rust (native/)**
```bash
cd native
cargo test
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo build --release  # binary at target/release/filament-mat-lsp
```

## Local CI Verification (Run Before Push)

Mirror of `.github/workflows/ci.yml`. Run these before every commit to ensure CI will pass:

```bash
# Full verification (Rust + TypeScript)
npm run check

# Rust only
npm run check:rust
# Equivalent to: cd native && cargo fmt --all -- --check && cargo clippy -- -D warnings && cargo check --all && cargo test --all

# TypeScript only
npm run check:ts
# Equivalent to: cd client && npx tsc --noEmit

# Auto-fix Rust issues
npm run fix
# Equivalent to: cd native && cargo fmt --all && cargo clippy --fix --allow-dirty -- -D warnings
```

**Toolchain note**: CI uses nightly Rust (`dtolnay/rust-toolchain@nightly`). If you encounter format/clippy discrepancies, ensure your local toolchain is nightly: `rustup default nightly`.

## Testing & Debugging

- **VS Code extension**: Open repo in VS Code, press `F5` to launch Extension Development Host. The client will auto-build the Rust binary if not present.
- **No automated integration tests** — `test/` contains only sample `.mat` files.
- **Rust unit tests**: `cd native && cargo test`

## CI / Verification

GitHub Actions (`.github/workflows/ci.yml`):
1. Rust checks on `windows-latest`
2. TypeScript type check on `windows-latest`
3. Release artifacts built for multiple platforms on tag push

## Pre-commit / Pre-push Policy

**Must follow**: Before executing `git commit` or `git push`, you must automatically run CI checks and confirm they pass before proceeding with git operations.

Execution flow:
1. First run: `npm run check`
2. If it fails:
   - Run `npm run fix` to attempt auto-fixes
   - Run `npm run check` again
   - If it still fails, report the specific errors to the user and **do not execute git commit/push**
3. Only after all checks pass, proceed with the user's requested git operation
4. Remind the user to check for secret files (.env, credentials.json, etc.) before committing

---

## Important Notes

- **Binary name**: `filament-mat-lsp` (macOS/Linux), `filament-mat-lsp.exe` (Windows)
- **Binary discovery priority**:
  1. Extension global storage (cached downloaded binary)
  2. `native/target/release/` (development build)
  3. GitHub Releases (fallback download)
- **No ESLint config**: `npm run lint` references `eslint` but there is no `.eslintrc` or `eslint.config` file in the repo.
- **Prettier config**: `.prettierrc` present (semi, trailingComma es5, double quotes, 100 width, 2-space tabs).
- **Root tsconfig**: Only includes `client/src`. Server directory removed.
