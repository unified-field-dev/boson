# Boson extraction (Zone A)

Upstream-only playbook for [deathbreakfast/boson](https://github.com/deathbreakfast/boson).

## Zone A crates

| Phase | Crate | Status |
|-------|-------|--------|
| 1 | `boson-core`, `boson-telemetry` | **shipped** @ `v0.1.0` |
| 2 | `boson-backend-mem` | **shipped** @ `v0.1.1` |
| 3 | `boson-runtime`, `boson-axum`, `boson`, `boson-testkit` (mem slice) | planned |
| 5 | `boson-e2e`, `boson-bench` | planned |

## Ports

- **`QueueBackend`** — async persistence for jobs, runs, task config, leases (`boson-core`)
- **`QueueRouter`** — register named backends at host boot (`boson-core`)
- **`ExecutionContext` / `ExecutionContextFactory`** — identity at handler boundary (`boson-core`)
- **`OpsLog`** — self-metrics and ops events (`boson-telemetry`)

## Build guardrails

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-boson-extract
cargo test -p boson-core
cargo test -p boson-telemetry
cargo test -p boson-backend-mem
```

Scope narrowly — one `cargo` command at a time on constrained hosts.

## Phase 1 verify

```bash
cd ~/boson
export CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target-boson-extract
cargo check -p boson-core
cargo test -p boson-core
cargo check -p boson-telemetry
cargo test -p boson-telemetry
cargo clippy -p boson-core --all-targets -- -D warnings
cargo clippy -p boson-telemetry --all-targets -- -D warnings
```

## Phase 2 exit criteria (shipped @ `v0.1.1`)

`boson-backend-mem` implements all 26 `QueueBackend` methods with integration tests for enqueue, claim, run lifecycle, rate limits, and lease contention. Bootstrap helper [`install_default_mem_backend`](boson-backend-mem/src/bootstrap.rs) registers on `QueueRouter`.

Verify:

```bash
cd ~/boson
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-boson-extract

cargo check -p boson-backend-mem
cargo test -p boson-backend-mem
cargo clippy -p boson-backend-mem --all-targets -- -D warnings
cargo doc -p boson-backend-mem --no-deps

rg -i 'valence|spectra|surreal|photon|web-app-template' boson-backend-mem
# expect: no matches
```

Sentrux baseline: `boson-backend-mem` `quality_signal` **7219** (see crate `QUALITY.md`).

## Third-party adapter checklist

1. Implement `QueueBackend` using portable DTOs from `boson-core`.
2. Publish as `yourorg-boson-backend-{substrate}`.
3. Optional: bootstrap helper that registers on `QueueRouter`.
4. Document required worker mode and lease TTL expectations.
