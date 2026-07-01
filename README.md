# Boson (Zone A)

Composable async work engine — portable queue ports, worker modes, and telemetry hooks.

## Crates

| Crate | Role |
|-------|------|
| `boson-core` | `QueueBackend`, `QueueRouter`, portable DTOs, `ExecutionContext` ports |
| `boson-telemetry` | `OpsLog` trait; `NoOpsLog` / `ConsoleOpsLog` |

Later phases add `boson-backend-mem`, `boson-runtime`, `boson-axum`, `boson-testkit`, `boson-e2e`, and `boson-bench`. See [`EXTRACTION.md`](EXTRACTION.md).

## Third-party adapters

Implement [`QueueBackend`](boson-core/src/backend/queue_backend.rs) against DTOs from `boson-core`, then compose:

```rust
use std::sync::Arc;

let boson = Boson::builder()
    .queue_backend(Arc::new(your_backend))
    .execution_context_factory(your_factory)
    .build()?;
```

(`Boson` / builder ship in `boson-runtime` — Phase 3.)

## Build

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-boson-extract
cargo test -p boson-core -p boson-telemetry
```

## Extraction

Phased upstream work: [`EXTRACTION.md`](EXTRACTION.md).
