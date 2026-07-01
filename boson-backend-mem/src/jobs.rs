//! Job persistence helpers for the in-memory backend.

use boson_core::{
    BosonError, Job, JobEnqueueDisposition, JobStatus, Result, TaskConfig,
};

use crate::enqueue_rate::EnqueueRateLimiter;
use crate::store::Inner;

/// Persist or replace a job row.
pub fn upsert_job(inner: &mut Inner, job: &Job) -> Result<()> {
    inner.jobs.insert(job.job_id.clone(), job.clone());
    Ok(())
}

/// Find non-terminal job id by idempotency key.
pub fn find_nonterminal_by_idempotency_key(inner: &Inner, key: &str) -> Result<Option<String>> {
    if key.is_empty() {
        return Ok(None);
    }
    for job in inner.jobs.values() {
        if job.idempotency_key.as_deref() == Some(key)
            && matches!(job.status, JobStatus::Queued | JobStatus::Running)
        {
            return Ok(Some(job.job_id.clone()));
        }
    }
    Ok(None)
}

/// Count active (`queued` + `running`) jobs for one task.
pub fn count_active_jobs_for_task(inner: &Inner, task_name: &str) -> Result<u32> {
    let count = inner
        .jobs
        .values()
        .filter(|j| {
            j.task_name == task_name
                && matches!(j.status, JobStatus::Queued | JobStatus::Running)
        })
        .count();
    Ok(count as u32)
}

/// Enforce policies and insert a job.
pub fn enqueue_with_policies(
    inner: &mut Inner,
    rate_limiter: &EnqueueRateLimiter,
    job: Job,
    task_config: &TaskConfig,
) -> Result<(String, JobEnqueueDisposition)> {
    if let Some(ref key) = job.idempotency_key {
        if !key.is_empty() {
            if let Some(existing) = find_nonterminal_by_idempotency_key(inner, key)? {
                return Ok((existing, JobEnqueueDisposition::ReusedIdempotent));
            }
        }
    }

    let policy = &task_config.rate_limit_policy;
    if policy.max_in_flight > 0 {
        let count = count_active_jobs_for_task(inner, &job.task_name)?;
        if count >= policy.max_in_flight {
            return Err(BosonError::RateLimited(job.task_name.clone()));
        }
    }

    if policy.max_enqueue_per_second > 0
        && !rate_limiter.try_record(&job.task_name, policy.max_enqueue_per_second)
    {
        return Err(BosonError::RateLimited(job.task_name.clone()));
    }

    let job_id = job.job_id.clone();
    upsert_job(inner, &job)?;
    Ok((job_id, JobEnqueueDisposition::InsertedNew))
}

/// Load one job.
pub fn get_job(inner: &Inner, job_id: &str) -> Result<Option<Job>> {
    Ok(inner.jobs.get(job_id).cloned())
}

/// List jobs with optional status filter and pagination.
pub fn list_jobs(
    inner: &Inner,
    status_filter: Option<JobStatus>,
    offset: usize,
    limit: usize,
) -> Result<Vec<Job>> {
    let mut jobs: Vec<Job> = inner
        .jobs
        .values()
        .filter(|j| status_filter.is_none_or(|s| j.status == s))
        .cloned()
        .collect();
    jobs.sort_by_key(|j| j.created_at);
    Ok(jobs.into_iter().skip(offset).take(limit).collect())
}

/// Cancel a job if still active.
pub fn cancel_job_if_active(inner: &mut Inner, job_id: &str) -> Result<()> {
    let Some(job) = inner.jobs.get_mut(job_id) else {
        return Err(BosonError::JobNotFound(job_id.to_string()));
    };
    if matches!(job.status, JobStatus::Queued | JobStatus::Running) {
        job.status = JobStatus::Canceled;
    }
    Ok(())
}

/// Atomically claim a queued job.
pub fn try_claim_job(inner: &mut Inner, job_id: &str) -> Result<Option<Job>> {
    let Some(job) = inner.jobs.get_mut(job_id) else {
        return Ok(None);
    };
    if job.status != JobStatus::Queued {
        return Ok(None);
    }
    job.status = JobStatus::Running;
    Ok(Some(job.clone()))
}

/// Revert a running job to queued.
pub fn revert_job_to_queued(inner: &mut Inner, job_id: &str) -> Result<()> {
    let Some(job) = inner.jobs.get_mut(job_id) else {
        return Ok(());
    };
    if job.status == JobStatus::Running {
        job.status = JobStatus::Queued;
    }
    Ok(())
}

/// Distinct pool names among queued jobs.
pub fn distinct_pools_queued(inner: &Inner) -> Result<Vec<String>> {
    let mut pools: Vec<String> = inner
        .jobs
        .values()
        .filter(|j| j.status == JobStatus::Queued)
        .map(|j| j.pool.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    pools.sort();
    Ok(pools)
}

/// Queued jobs for one pool sorted by priority then created time.
pub fn list_queued_for_pool_sorted(inner: &Inner, pool: &str, limit: usize) -> Result<Vec<Job>> {
    let mut jobs: Vec<Job> = inner
        .jobs
        .values()
        .filter(|j| j.status == JobStatus::Queued && j.pool == pool)
        .cloned()
        .collect();
    jobs.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| a.created_at.cmp(&b.created_at)));
    jobs.truncate(limit);
    Ok(jobs)
}

/// Count jobs optionally filtered by status.
pub fn count_jobs(inner: &Inner, status_filter: Option<JobStatus>) -> Result<u64> {
    let count = inner
        .jobs
        .values()
        .filter(|j| status_filter.is_none_or(|s| j.status == s))
        .count();
    Ok(count as u64)
}

/// Count jobs for one task optionally filtered by status.
pub fn count_jobs_for_task(
    inner: &Inner,
    task_name: &str,
    status: Option<JobStatus>,
) -> Result<u64> {
    let count = inner
        .jobs
        .values()
        .filter(|j| {
            j.task_name == task_name && status.is_none_or(|s| j.status == s)
        })
        .count();
    Ok(count as u64)
}
