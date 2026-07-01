//! Boson operations telemetry port (replaces direct host telemetry in Zone A runtime).
//!
//! **Audience:** Integrators and operators installing self-metrics at server boot.
//!
//! ## Stack position
//!
//! ```text
//! boson-runtime → calls ops_log() → host may install a product telemetry adapter at boot
//! ```
//!
//! ## Entry points
//!
//! - [`OpsLog`] — counters, gauges, structured events
//! - [`install_ops_log`] — process-wide install at boot
//! - [`NoOpsLog`] / [`ConsoleOpsLog`] — built-in adapters
//!
//! UC1/UC3 metric names (`boson_tasks_enqueued`, `boson_task_log`, …) are documented on [`OpsLog`];
//! wiring lands in product adapters (Phase 4+).

#![deny(missing_docs)]

mod console;
mod global;
mod noop;

pub use console::ConsoleOpsLog;
pub use global::{install_ops_log, ops_log, ops_log_from_env};
pub use noop::NoOpsLog;

/// Structured ops metrics and events for enqueue, runs, leases, and runtime health.
pub trait OpsLog: Send + Sync {
    /// Increment a counter with optional labels.
    fn record_counter(&self, name: &str, labels: &[(&str, &str)], value: f64);

    /// Set a gauge with optional labels.
    fn record_gauge(&self, name: &str, labels: &[(&str, &str)], value: f64);

    /// Emit a structured diagnostic event (e.g. `boson_task_log`, `boson_handler_error`).
    fn log_event(&self, name: &str, payload: &serde_json::Value);
}

#[cfg(test)]
mod tests {
    use super::{ConsoleOpsLog, NoOpsLog, OpsLog};

    #[test]
    fn noop_ops_log_is_silent() {
        let log = NoOpsLog;
        log.record_counter("c", &[], 1.0);
        log.record_gauge("g", &[], 2.0);
        log.log_event("e", &serde_json::json!({}));
    }

    #[test]
    fn console_ops_log_does_not_panic() {
        let log = ConsoleOpsLog;
        log.record_counter("boson_tasks_enqueued", &[("task_name", "t")], 1.0);
    }
}
