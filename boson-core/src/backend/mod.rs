//! Queue persistence trait and related types.

mod keyset;
mod queue_backend;

pub use keyset::{page_jobs_after, page_runs_after, JobKey, RunKey};
pub use queue_backend::{
    default_backend_from_global, JobEnqueueDisposition, QueueBackend, DEFAULT_BACKEND_NAME,
};
