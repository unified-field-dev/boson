# boson-telemetry

`OpsLog` telemetry port for Boson self-metrics and ops events.

## Adapters (this crate)

| Adapter | Notes |
|---------|-------|
| `ConsoleOpsLog` | stderr / structured console |
| `NoOpsLog` | default no-op |

Hosts may install other `OpsLog` implementations from separate adapter crates at boot.

## Environment

- `BOSON_TELEMETRY=off|console` (default `console`)

## Status

Phase 1 shipped @ [deathbreakfast/boson](https://github.com/deathbreakfast/boson) `v0.1.0`.
