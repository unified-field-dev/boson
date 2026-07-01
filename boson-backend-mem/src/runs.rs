//! Run persistence helpers for the in-memory backend.

use boson_core::{Result, Run, RunStatus, TaskRunStats};
use chrono::{DateTime, Utc};

use crate::store::Inner;

/// Persist or replace a run row.
pub fn upsert_run(inner: &mut Inner, run: &Run) -> Result<()> {
    inner.runs.insert(run.run_id.clone(), run.clone());
    Ok(())
}

/// Load one run.
pub fn get_run(inner: &Inner, run_id: &str) -> Result<Option<Run>> {
    Ok(inner.runs.get(run_id).cloned())
}

/// List runs with optional job filter and pagination.
pub fn list_runs(
    inner: &Inner,
    job_id_filter: Option<&str>,
    offset: usize,
    limit: usize,
) -> Result<Vec<Run>> {
    let mut runs: Vec<Run> = inner
        .runs
        .values()
        .filter(|r| job_id_filter.is_none_or(|jid| r.job_id == jid))
        .cloned()
        .collect();
    runs.sort_by_key(|r| std::cmp::Reverse(r.started_at));
    Ok(runs.into_iter().skip(offset).take(limit).collect())
}

/// Mark a run terminal with outcome fields.
pub fn finish_run(
    inner: &mut Inner,
    run_id: &str,
    status: RunStatus,
    duration_ms: Option<i64>,
    error_message: Option<String>,
) -> Result<()> {
    let Some(run) = inner.runs.get_mut(run_id) else {
        return Ok(());
    };
    run.status = status;
    run.finished_at = Some(Utc::now());
    run.duration_ms = duration_ms;
    run.error_message = error_message;
    Ok(())
}

/// Count runs optionally filtered by job id.
pub fn count_runs(inner: &Inner, job_id_filter: Option<&str>) -> Result<u64> {
    let count = inner
        .runs
        .values()
        .filter(|r| job_id_filter.is_none_or(|jid| r.job_id == jid))
        .count();
    Ok(count as u64)
}

/// Count runs with `started_at >= since`.
pub fn count_runs_since(inner: &Inner, since: DateTime<Utc>) -> Result<u64> {
    let count = inner
        .runs
        .values()
        .filter(|r| r.started_at >= since)
        .count();
    Ok(count as u64)
}

/// Aggregate run totals for one task.
pub fn task_run_stats(inner: &Inner, task_name: &str) -> Result<TaskRunStats> {
    let matching: Vec<&Run> = inner
        .runs
        .values()
        .filter(|r| r.task_name == task_name)
        .collect();
    let runs_total = matching.len() as u32;
    let success_count = matching
        .iter()
        .filter(|r| r.status == RunStatus::Success)
        .count() as u32;
    Ok(TaskRunStats {
        runs_total,
        success_count,
    })
}
