---
description: Auto-fix Rust formatting and clippy warnings, then re-run checks
model: anthropic/claude-sonnet-4-5
---

Run Rust auto-fix tools, then verify the results.

1. Run auto-fix:
   !`npm run fix`
   (Equivalent to: cd native && cargo fmt --all && cargo clippy --fix --allow-dirty -- -D warnings)

2. Re-run full checks after fix:
   !`npm run check`

3. If there are still errors:
   - Check if nightly toolchain is needed: `rustup default nightly`
   - Analyze remaining errors and provide manual fix suggestions

Note:
- This agent modifies files. Ensure only formatting/clippy related changes are made
- Do not modify functional logic; only fix formatting and lint warnings
