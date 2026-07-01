//! Distributed lease helpers for the in-memory backend.

use boson_core::Result;
use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::store::{LeaseRecord, Inner};

/// Remove expired leases for one job id.
fn purge_expired_for_job(inner: &mut Inner, job_id: &str) {
    let now = Utc::now();
    inner.leases.retain(|_, lease| {
        !(lease.job_id == job_id && lease.expires_at <= now)
    });
}

/// Returns true when an active lease exists for `job_id`.
fn has_active_lease(inner: &Inner, job_id: &str) -> bool {
    let now = Utc::now();
    inner
        .leases
        .values()
        .any(|l| l.job_id == job_id && l.expires_at > now)
}

/// Attempt to claim a run lease for `job_id`.
pub fn try_claim_run_lease(
    inner: &mut Inner,
    job_id: &str,
    _worker_id: &str,
    ttl_secs: i64,
) -> Result<Option<String>> {
    purge_expired_for_job(inner, job_id);
    if has_active_lease(inner, job_id) {
        return Ok(None);
    }
    let now = Utc::now();
    let lease_id = Uuid::new_v4().to_string();
    inner.leases.insert(
        lease_id.clone(),
        LeaseRecord {
            job_id: job_id.to_string(),
            expires_at: now + Duration::seconds(ttl_secs),
        },
    );
    Ok(Some(lease_id))
}

/// Extend lease TTL for a held lease.
pub fn extend_lease(inner: &mut Inner, lease_id: &str, ttl_secs: i64) -> Result<()> {
    let now = Utc::now();
    if let Some(lease) = inner.leases.get_mut(lease_id) {
        lease.expires_at = now + Duration::seconds(ttl_secs);
    }
    Ok(())
}

/// Release a held lease.
pub fn release_lease(inner: &mut Inner, lease_id: &str) -> Result<()> {
    inner.leases.remove(lease_id);
    Ok(())
}

/// Expired leases as `(lease_id, job_id)`.
pub fn expired_lease_job_pairs(inner: &Inner) -> Result<Vec<(String, String)>> {
    let now = Utc::now();
    Ok(inner
        .leases
        .iter()
        .filter(|(_, l)| l.expires_at <= now)
        .map(|(id, l)| (id.clone(), l.job_id.clone()))
        .collect())
}
