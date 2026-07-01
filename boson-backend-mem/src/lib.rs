//! In-memory [`QueueBackend`](boson_core::QueueBackend) adapter.
//!
//! **Audience:** Backend engineers and test authors who need queue persistence without
//! network I/O or host-specific storage.
//!
//! ## Stack position
//!
//! ```text
//! boson-testkit / inline tests → boson-backend-mem → boson-core
//! ```
//!
//! ## Entry points
//!
//! - [`MemQueueBackend`] — implementor of all queue persistence operations in-process
//! - [`install_default_mem_backend`] — register on global [`QueueRouter`](boson_core::QueueRouter)
//!
//! ## Prerequisites
//!
//! Requires Phase 1 `boson-core` DTOs and trait contracts. Worker runtime integration
//! lands in `boson-runtime` (Phase 3).

#![deny(missing_docs)]

mod bootstrap;
mod enqueue_rate;
mod error;
mod jobs;
mod leases;
mod mem_queue_backend;
mod runs;
mod store;
mod task_config;

pub use bootstrap::install_default_mem_backend;
pub use mem_queue_backend::MemQueueBackend;
