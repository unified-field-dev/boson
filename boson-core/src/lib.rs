//! Portable types, [`QueueBackend`] port, router, and identity hooks.
//!
//! **Audience:** Backend and framework authors implementing queue persistence adapters.
//!
//! ## Stack position
//!
//! ```text
//! boson-runtime (Phase 3) → boson-core ← boson-backend-* adapters
//! ```
//!
//! ## Entry points
//!
//! - [`QueueBackend`] — persistence port for jobs, runs, config, leases
//! - [`QueueRouter`] — register named backends at host boot
//! - [`ExecutionContextFactory`] — rebuild handler identity from captured JSON
//! - [`Job`], [`Run`], [`TaskConfig`] — portable DTOs
//!
//! Hosts inject a `QueueBackend` implementation at build time; this crate has no I/O.

#![deny(missing_docs)]

pub mod backend;
pub mod error;
pub mod identity;
pub mod mode;
pub mod models;
pub mod router;

pub use backend::{default_backend_from_global, JobEnqueueDisposition, QueueBackend};
pub use error::{BosonError, IdentityError, Result};
pub use identity::{ExecutionContext, ExecutionContextFactory};
pub use mode::BosonMode;
pub use models::{
    Job, JobStatus, RateLimitPolicy, RetryPolicy, Run, RunStatus, TaskConfig, TaskRunStats,
};
pub use router::QueueRouter;
