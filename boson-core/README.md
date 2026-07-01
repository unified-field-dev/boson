# boson-core

Portable types, [`QueueBackend`](src/backend/queue_backend.rs) port, router, errors, and worker modes.

## Role

- `Job`, `Run`, `TaskConfig`, status enums
- **`QueueBackend`** — stable async trait for queue persistence
- **`QueueRouter`** — register named backends at host boot
- `BosonMode`, `BosonError`
- `ExecutionContext`, `ExecutionContextFactory` — identity port uses JSON `actor_json`

Third-party crates implement **`QueueBackend`** only against DTOs exported here.

## Status

Phase 1 shipped @ [deathbreakfast/boson](https://github.com/deathbreakfast/boson) `v0.1.0`.
