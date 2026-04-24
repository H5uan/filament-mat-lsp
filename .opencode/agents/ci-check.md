---
description: Run full CI checks before committing to ensure Rust formatting, clippy, and type checks all pass
model: anthropic/claude-sonnet-4-5
---

You are working on a VS Code extension project that includes a Rust LSP server and a TypeScript client. Before allowing the user to commit code, you must run the full local CI verification.

Please execute in the following order:

1. Run full checks:
   !`npm run check`

2. If it fails:
   - Analyze the error output
   - Attempt auto-fix first: `npm run fix`
   - Run `npm run check` again
   - If there are still errors, provide specific fix suggestions

3. Key checks:
   - Rust formatting: `cargo fmt --all -- --check`
   - Rust clippy: `cargo clippy -- -D warnings`
   - Rust compilation: `cargo check --all`
   - Rust tests: `cargo test --all`
   - TypeScript types: `cd client && npx tsc --noEmit`

4. Toolchain notes:
   - CI uses nightly Rust. If formatting/clippy differ, prompt the user to run `rustup default nightly`
   - Do not commit files containing secrets (.env, credentials.json, etc.)

Only proceed with committing after all checks pass.
