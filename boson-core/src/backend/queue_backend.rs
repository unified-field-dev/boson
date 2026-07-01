//! [`QueueBackend`] — injectable queue persistence port.

use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::error::Result;
use crate::models::{Job, JobStatus, Run, RunStatus, TaskConfig, TaskRunStats};

/// Default router key for single-backend hosts.
pub const DEFAULT_BACKEND_NAME: &str = "default";

/// Whether [`QueueBackend::enqueue_with_policies`] inserted or reused a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobEnqueueDisposition {
    /// A new job row was inserted.
    InsertedNew,
    /// An existing non-terminal job matched the idempotency key.
    ReusedIdempotent,
}

/// Stable async port for queue persistence (jobs, runs, config, leases).
///
/// # Contract
///
/// - Implementations must be `Send + Sync`; hosts hold `Arc<dyn QueueBackend>`.
/// - Idempotency: when `job.idempotency_key` is set, `enqueue_with_policies` returns
///   [`ReusedIdempotent`](JobEnqueueDisposition::ReusedIdempotent) with the existing job id
///   if a non-terminal job already exists for that key.
/// - Claim: `try_claim_job` must atomically transition `queued` → `running` or return `None`.
/// - Leases: `try_claim_run_lease` returns `None` when another worker holds an active lease.
#[async_trait]
pub trait QueueBackend: Send + Sync + Debug {
    // --- Jobs ---

    /// Persist or update a job row.
    async fn upsert_job(&self, job: &Job) -> Result<()>;

    /// Insert job with idempotency semantics (see trait-level contract).
    async fn enqueue_with_policies(
        &self,
        job: Job,
        task_config: &TaskConfig,
    ) -> Result<(String, JobEnqueueDisposition)>;

    /// Load one job by id.
    async fn get_job(&self, job_id: &str) -> Result<Option<Job>>;

    /// List jobs with optional status filter and pagination.
    async fn list_jobs(
        &self,
        status_filter: Option<JobStatus>,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Job>>;

    /// Cancel a job if it is still active (`queued` or `running`).
    async fn cancel_job_if_active(&self, job_id: &str) -> Result<()>;

    /// Atomically claim a queued job for execution.
    async fn try_claim_job(&self, job_id: &str) -> Result<Option<Job>>;

    /// Revert a job to `queued` (retry path).
    async fn revert_job_to_queued(&self, job_id: &str) -> Result<()>;

    /// Distinct pool names with queued jobs.
    async fn distinct_pools_queued(&self) -> Result<Vec<String>>;

    /// Queued jobs for one pool, sorted by priority then created time.
    async fn list_queued_for_pool_sorted(&self, pool: &str, limit: usize) -> Result<Vec<Job>>;

    /// Count jobs, optionally filtered by status.
    async fn count_jobs(&self, status_filter: Option<JobStatus>) -> Result<u64>;

    /// Count jobs for one task, optionally filtered by status.
    async fn count_jobs_for_task(
        &self,
        task_name: &str,
        status: Option<JobStatus>,
    ) -> Result<u64>;

    /// Count active (`queued` + `running`) jobs for rate-limit checks.
    async fn count_active_jobs_for_task(&self, task_name: &str) -> Result<u32>;

    /// Find non-terminal job by idempotency key.
    async fn find_nonterminal_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<String>>;

    // --- Runs ---

    /// Persist or update a run row.
    async fn upsert_run(&self, run: &Run) -> Result<()>;

    /// Load one run by id.
    async fn get_run(&self, run_id: &str) -> Result<Option<Run>>;

    /// List runs with optional job filter and pagination.
    async fn list_runs(
        &self,
        job_id_filter: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Run>>;

    /// Mark a run terminal with outcome fields.
    async fn finish_run(
        &self,
        run_id: &str,
        status: RunStatus,
        duration_ms: Option<i64>,
        error_message: Option<String>,
    ) -> Result<()>;

    /// Count runs, optionally filtered by job id.
    async fn count_runs(&self, job_id_filter: Option<&str>) -> Result<u64>;

    /// Count runs with `started_at >= since`.
    async fn count_runs_since(&self, since: DateTime<Utc>) -> Result<u64>;

    /// Aggregate run totals for one task.
    async fn task_run_stats(&self, task_name: &str) -> Result<TaskRunStats>;

    // --- Task config ---

    /// Load task config by name.
    async fn get_task_config(&self, task_name: &str) -> Result<Option<TaskConfig>>;

    /// Persist task config.
    async fn upsert_task_config(&self, config: &TaskConfig) -> Result<()>;

    // --- Leases (distributed workers) ---

    /// Attempt to claim a run lease for `job_id` (stored as lease task id).
    async fn try_claim_run_lease(
        &self,
        job_id: &str,
        worker_id: &str,
        ttl_secs: i64,
    ) -> Result<Option<String>>;

    /// Extend lease TTL for a held lease.
    async fn extend_lease(&self, lease_id: &str, ttl_secs: i64) -> Result<()>;

    /// Release a held lease.
    async fn release_lease(&self, lease_id: &str) -> Result<()>;

    /// Expired leases as `(lease_record_id, job_id)`.
    async fn expired_lease_job_pairs(&self) -> Result<Vec<(String, String)>>;
}

/// Resolve the default backend from the process-global [`QueueRouter`](crate::QueueRouter).
///
/// Returns [`BosonError::Internal`](crate::BosonError::Internal) when the global router
/// was not installed via [`QueueRouter::set_global`](crate::QueueRouter::set_global).
pub fn default_backend_from_global() -> Result<Arc<dyn QueueBackend>> {
    let router = crate::QueueRouter::try_global().ok_or_else(|| {
        crate::BosonError::Internal("QueueRouter::set_global was not called".into())
    })?;
    router.resolve(DEFAULT_BACKEND_NAME)
}
