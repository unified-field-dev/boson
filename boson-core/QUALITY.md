# boson-core Quality Gates

Sentrux MCP (`user-sentrux`) structure-health signal for this crate.

## Baseline (Phase 1)

- `scan(path="/home/seanorourke/boson/boson-core")` → `quality_signal`: **7220**
- Tests: `cargo test -p boson-core`
- Clippy: `cargo clippy -p boson-core --all-targets -- -D warnings`

## Local commands

```bash
cd ~/boson
export CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target-boson-extract
cargo test -p boson-core
cargo clippy -p boson-core --all-targets -- -D warnings
cargo doc -p boson-core --no-deps
```

## Targets

- Preserve or improve Sentrux `quality_signal`
- Zero circular dependencies; no file > 450 LOC
- `#![deny(missing_docs)]` on public API
