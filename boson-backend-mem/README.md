# boson-backend-mem

In-memory [`QueueBackend`](https://github.com/deathbreakfast/boson) adapter (tests, CI, testkit default).

## Role

Implements `QueueBackend` using portable DTOs from `boson-core` — no network I/O or host-specific persistence.

Used by:

- `boson-testkit` default `BootstrapSession` (Phase 3)
- Zone A inline tests and `boson-e2e` CI slice (Phase 5)
- `boson-bench` `backend=mem` campaigns (Phase 5)

## Compose

```rust
use std::sync::Arc;
use boson_backend_mem::MemQueueBackend;
use boson_core::{BosonMode, QueueBackend, TaskConfig};

let backend = Arc::new(MemQueueBackend::new());
// Phase 3: Boson::builder().queue_backend(backend).mode(BosonMode::Local).build()?;
let config = TaskConfig::default_for("my_task");
```

## Bootstrap

```rust
use boson_backend_mem::install_default_mem_backend;
use boson_core::default_backend_from_global;

let _backend = install_default_mem_backend();
let resolved = default_backend_from_global()?;
```

## Facade feature

Enabled via `boson` crate feature `mem` (forwards this adapter) — lands Phase 3.

## Status

**Phase 2 shipped** @ upstream `v0.1.1`.
