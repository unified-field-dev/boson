//! Shared [`QueueBackend`] contract assertions.

use std::sync::Arc;

use boson_core::{
    Job, JobEnqueueDisposition, JobKey, JobStatus, QueueBackend, RateLimitPolicy, RunKey,
    RunStatus, TaskConfig,
};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;

/// Fixture environment for backend contract tests.
pub struct BackendEnv {
    /// Optional label for debugging.
    pub label: &'static str,
}

impl BackendEnv {
    /// New fixture env.
    #[must_use]
    pub const fn new(label: &'static str) -> Self {
        Self { label }
    }
}

fn task_config(task_name: &str) -> TaskConfig {
    TaskConfig::default_for(task_name)
}

fn task_config_with_rate_limit(task_name: &str, max_in_flight: u32, max_eps: u32) -> TaskConfig {
    let mut config = TaskConfig::default_for(task_name);
    config.rate_limit_policy = RateLimitPolicy {
        max_in_flight,
        max_enqueue_per_second: max_eps,
    };
    config
}

fn sample_job(
    task_name: &str,
    pool: &str,
    priority: i32,
    idempotency_key: Option<&str>,
) -> boson_core::Job {
    boson_core::Job::new(
        task_name,
        json!({"label": "test"}),
        json!({}),
        priority,
        pool,
        0,
        idempotency_key.map(str::to_string),
    )
}

async fn enqueue(
    backend: &dyn QueueBackend,
    job: boson_core::Job,
    config: &TaskConfig,
) -> (String, JobEnqueueDisposition) {
    backend
        .enqueue_with_policies(job, config)
        .await
        .expect("enqueue")
}

/// Asserts enqueue inserts and lists the job.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn enqueue_inserts_and_lists(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    let (job_id, disp) = enqueue(&*b, job, &config).await;
    assert_eq!(disp, JobEnqueueDisposition::InsertedNew);
    let listed = b.list_jobs(Some(JobStatus::Queued), 0, 10).await.unwrap();
    assert!(listed.iter().any(|j| j.job_id == job_id));
}

/// Asserts idempotency reuses non-terminal jobs.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn idempotency_reuses_nonterminal(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config("echo");
    let job1 = sample_job("echo", "global", 1, Some("idem-1"));
    let (id1, _) = enqueue(&*b, job1, &config).await;
    let job2 = sample_job("echo", "global", 1, Some("idem-1"));
    let (id2, disp) = enqueue(&*b, job2, &config).await;
    assert_eq!(disp, JobEnqueueDisposition::ReusedIdempotent);
    assert_eq!(id1, id2);
}

/// Asserts atomic job claim semantics.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn try_claim_atomic(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    let (job_id, _) = enqueue(&*b, job, &config).await;
    assert!(b.try_claim_job(&job_id).await.unwrap().is_some());
    assert!(b.try_claim_job(&job_id).await.unwrap().is_none());
}

/// Asserts two concurrent claimers produce exactly one success.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn try_claim_parallel(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    let (job_id, _) = enqueue(&*b, job, &config).await;
    let b1 = Arc::clone(&b);
    let b2 = Arc::clone(&b);
    let id1 = job_id.clone();
    let id2 = job_id.clone();
    let (r1, r2) = tokio::join!(b1.try_claim_job(&id1), b2.try_claim_job(&id2));
    let c1 = r1.expect("claimer a");
    let c2 = r2.expect("claimer b");
    let wins = usize::from(c1.is_some()) + usize::from(c2.is_some());
    assert_eq!(wins, 1, "exactly one parallel claimer must win");
    assert!(
        b.try_claim_job(&job_id).await.unwrap().is_none(),
        "third claim must fail"
    );
}

/// Asserts claim CAS keys off structured job status, not a substring in `params_json`.
///
/// Regression for Redis substring `"status":"queued"` matching inside attacker-controlled params.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn claim_ignores_status_in_params(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config("echo");
    let mut job = sample_job("echo", "global", 1, None);
    let poisoned = json!({
        "note": "attacker \"status\":\"queued\" in params",
        "nested": { "status": "queued" },
    });
    job.params_json = poisoned.clone();
    let (job_id, _) = enqueue(&*b, job, &config).await;
    let claimed = b
        .try_claim_job(&job_id)
        .await
        .unwrap()
        .expect("first claim must succeed despite poisoned params");
    assert_eq!(claimed.status, JobStatus::Running);
    assert_eq!(claimed.params_json, poisoned);
    assert!(
        b.try_claim_job(&job_id).await.unwrap().is_none(),
        "second claim must fail"
    );
    let loaded = b.get_job(&job_id).await.unwrap().expect("job");
    assert_eq!(loaded.status, JobStatus::Running);
    assert_eq!(loaded.params_json, poisoned);
}

/// Asserts pool priority ordering for queued jobs.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn pool_priority_order(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config("echo");
    let low = sample_job("echo", "workers", 1, None);
    let high = sample_job("echo", "workers", 5, None);
    enqueue(&*b, high, &config).await;
    enqueue(&*b, low, &config).await;
    let queued = b.list_queued_for_pool_sorted("workers", 10).await.unwrap();
    assert_eq!(queued.len(), 2);
    assert!(queued[0].priority <= queued[1].priority);
}

/// Asserts max in-flight rate limit enforcement.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn max_in_flight_rate_limit(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config_with_rate_limit("echo", 1, 0);
    let job1 = sample_job("echo", "global", 1, None);
    enqueue(&*b, job1, &config).await;
    let job2 = sample_job("echo", "global", 1, None);
    let err = b.enqueue_with_policies(job2, &config).await.unwrap_err();
    assert!(matches!(err, boson_core::BosonError::RateLimited(_)));
}

/// Asserts max enqueue per second rate limit enforcement.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn max_enqueue_per_second(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config_with_rate_limit("echo", 0, 1);
    let job1 = sample_job("echo", "global", 1, None);
    enqueue(&*b, job1, &config).await;
    let job2 = sample_job("echo", "global", 1, None);
    let err = b.enqueue_with_policies(job2, &config).await.unwrap_err();
    assert!(matches!(err, boson_core::BosonError::RateLimited(_)));
}

/// Asserts run lifecycle persistence.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn run_lifecycle(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    let (job_id, _) = enqueue(&*b, job, &config).await;
    let run = boson_core::Run::new(&job_id, "echo", 1);
    let run_id = run.run_id.clone();
    b.upsert_run(&run).await.unwrap();
    b.finish_run(&run_id, RunStatus::Success, Some(10), None)
        .await
        .unwrap();
    let loaded = b.get_run(&run_id).await.unwrap().expect("run");
    assert_eq!(loaded.status, RunStatus::Success);
}

/// Asserts the backend can be installed as the process-global default and resolved.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn global_router_resolves(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    boson_core::QueueRouter::set_global(boson_core::QueueRouter::with_default(Arc::clone(&b)));
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    enqueue(&*b, job, &config).await;
    let resolved = boson_core::default_backend_from_global().unwrap();
    assert!(Arc::ptr_eq(&b, &resolved));
}

/// Asserts lease contention blocks a second worker claim.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn lease_contention(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let job_id = "job-1";
    let l1 = b
        .try_claim_run_lease(job_id, "worker-a", 120)
        .await
        .unwrap()
        .expect("first claim");
    assert!(b
        .try_claim_run_lease(job_id, "worker-b", 120)
        .await
        .unwrap()
        .is_none());
    b.release_lease(&l1).await.unwrap();
    assert!(b
        .try_claim_run_lease(job_id, "worker-b", 120)
        .await
        .unwrap()
        .is_some());
}

/// Asserts lease extension keeps the lease active.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn extend_lease_refreshes_ttl(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let lease_id = b
        .try_claim_run_lease("job-2", "worker-a", 60)
        .await
        .unwrap()
        .expect("claim");
    b.extend_lease(&lease_id, 300).await.unwrap();
    assert_eq!(b.expired_lease_job_pairs().await.unwrap(), Vec::new());
}

/// Asserts expired leases appear in expired pairs listing.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn expired_lease_pairs(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let lease_id = b
        .try_claim_run_lease("job-3", "worker-a", -1)
        .await
        .unwrap()
        .expect("claim with negative ttl => already expired");
    let pairs = b.expired_lease_job_pairs().await.unwrap();
    assert!(pairs
        .iter()
        .any(|(lid, jid)| lid == &lease_id && jid == "job-3"));
}

/// Asserts reaper semantics: expired lease + release/revert returns job to queued for reclaim.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn reclaim_expired_lease_requeues_job(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    let mut job = sample_job("reclaim-task", "global", 0, None);
    job.status = JobStatus::Running;
    let job_id = job.job_id.clone();
    b.upsert_job(&job).await.unwrap();
    let lease_id = b
        .try_claim_run_lease(&job_id, "worker-a", -1)
        .await
        .unwrap()
        .expect("expired lease");
    assert!(b
        .expired_lease_job_pairs()
        .await
        .unwrap()
        .iter()
        .any(|(lid, jid)| lid == &lease_id && jid == &job_id));
    b.release_lease(&lease_id).await.unwrap();
    b.revert_job_to_queued(&job_id).await.unwrap();
    let restored = b.get_job(&job_id).await.unwrap().expect("job");
    assert_eq!(restored.status, JobStatus::Queued);
    assert!(b.try_claim_job(&job_id).await.unwrap().is_some());
}

/// Whole-millisecond timestamp an hour in the past, so every engine stores it exactly
/// (Scylla keeps milliseconds, Postgres microseconds) and fresh enqueues sort after it.
fn keyset_base() -> DateTime<Utc> {
    let ms = Utc::now().timestamp_millis() - 3_600_000;
    Utc.timestamp_millis_opt(ms)
        .single()
        .expect("valid timestamp")
}

/// Offset `i / 3` milliseconds from `base`, so every timestamp is shared by three rows and the
/// id tie-break decides their order.
fn keyset_time(base: DateTime<Utc>, i: usize) -> DateTime<Utc> {
    base + chrono::Duration::milliseconds(i64::try_from(i / 3).expect("small index"))
}

/// Page `list_jobs_after` to the end and return every job in scan order.
///
/// `mid_scan` runs once after the third page.
async fn scan_jobs_after(
    b: &dyn QueueBackend,
    status: Option<JobStatus>,
    page_size: usize,
    mut mid_scan: impl FnMut() -> Option<Vec<Job>>,
) -> Vec<Job> {
    let mut out: Vec<Job> = Vec::new();
    let mut cursor: Option<JobKey> = None;
    let mut pages = 0_usize;
    loop {
        let page = b
            .list_jobs_after(status, cursor.as_ref(), page_size)
            .await
            .expect("list_jobs_after");
        assert!(page.len() <= page_size, "page exceeds limit");
        let Some(last) = page.last() else { break };
        cursor = Some(JobKey::from(last));
        out.extend(page);
        pages += 1;
        if pages == 3 {
            if let Some(jobs) = mid_scan() {
                for job in jobs {
                    enqueue(b, job, &task_config("keyset")).await;
                }
            }
        }
    }
    out
}

/// Checks one jobs scan: keys strictly increase, and among this test's jobs (`prefix`) the seeded
/// ones come first in `seeded` order, followed by exactly the `tail` ids in any order.
fn assert_jobs_scan(scanned: &[Job], prefix: &str, seeded: &[&str], tail: &[String], what: &str) {
    for pair in scanned.windows(2) {
        assert!(
            JobKey::from(&pair[0]).precedes(&pair[1]),
            "{what}: out of order: {} then {}",
            pair[0].job_id,
            pair[1].job_id
        );
    }
    let ids: Vec<&str> = scanned
        .iter()
        .map(|j| j.job_id.as_str())
        .filter(|id| id.starts_with(prefix))
        .collect();
    assert_eq!(
        ids.len(),
        seeded.len() + tail.len(),
        "{what}: each job once"
    );
    assert_eq!(
        ids[..seeded.len()],
        seeded[..],
        "{what}: seeded jobs in key order"
    );
    let mut got_tail: Vec<&str> = ids[seeded.len()..].to_vec();
    got_tail.sort_unstable();
    assert_eq!(got_tail, tail, "{what}: mid-scan enqueues at the end");
}

/// Asserts `list_jobs_after` pages 2,500 jobs in `created_at ASC, job_id ASC` order, once each,
/// applies the status filter, and puts jobs enqueued mid-scan at the end exactly once.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn keyset_jobs_pages_in_order(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    const TOTAL: usize = 2_500;
    const PAGE: usize = 100;
    const STATUSES: [JobStatus; 5] = [
        JobStatus::Queued,
        JobStatus::Running,
        JobStatus::Success,
        JobStatus::Failed,
        JobStatus::Canceled,
    ];
    let prefix = format!("ks-{}", uuid::Uuid::new_v4().simple());
    let base = keyset_base();
    let seeded: Vec<String> = (0..TOTAL).map(|i| format!("{prefix}-{i:05}")).collect();

    // Insert out of order (7 is coprime with 2,500) so insertion order cannot pass for sort order.
    for n in 0..TOTAL {
        let i = (n * 7) % TOTAL;
        let mut job = sample_job("keyset", "global", 1, None);
        job.job_id.clone_from(&seeded[i]);
        job.created_at = keyset_time(base, i);
        job.status = STATUSES[i % STATUSES.len()];
        b.upsert_job(&job).await.expect("upsert_job");
    }

    // Enqueued (status queued, created now) after the third page of the unfiltered scan.
    let new_ids: Vec<String> = (0..10).map(|k| format!("{prefix}-new-{k:02}")).collect();
    let mut pending = Some(
        new_ids
            .iter()
            .map(|id| {
                let mut job = sample_job("keyset", "global", 1, None);
                job.job_id.clone_from(id);
                job
            })
            .collect::<Vec<_>>(),
    );
    let scanned = scan_jobs_after(&*b, None, PAGE, || pending.take()).await;
    let all: Vec<&str> = seeded.iter().map(String::as_str).collect();
    assert_jobs_scan(&scanned, &prefix, &all, &new_ids, "unfiltered");

    for status in [JobStatus::Queued, JobStatus::Failed] {
        let filtered = scan_jobs_after(&*b, Some(status), PAGE, || None).await;
        assert!(filtered.iter().all(|j| j.status == status), "status filter");
        let expected: Vec<&str> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| STATUSES[i % STATUSES.len()] == status)
            .map(|(_, id)| *id)
            .collect();
        let tail: &[String] = if status == JobStatus::Queued {
            &new_ids
        } else {
            &[]
        };
        assert_jobs_scan(&filtered, &prefix, &expected, tail, &status.to_string());
    }
}

/// Asserts `list_runs_after` pages one job's 250 runs (of 500) in `started_at DESC, run_id DESC`
/// order, once each, and skips a run that starts mid-scan.
///
/// # Panics
///
/// Panics if backend operations fail or contract assertions are violated.
pub async fn keyset_runs_pages_in_order(b: Arc<dyn QueueBackend>, _env: &BackendEnv) {
    const TOTAL: usize = 500;
    const PAGE: usize = 40;
    let prefix = format!("ks-{}", uuid::Uuid::new_v4().simple());
    let job_even = format!("{prefix}-job-even");
    let job_odd = format!("{prefix}-job-odd");
    let base = keyset_base();
    let run_id = |i: usize| format!("{prefix}-{i:05}");

    for n in 0..TOTAL {
        let i = (n * 7) % TOTAL;
        let job = if i % 2 == 0 { &job_even } else { &job_odd };
        let mut run = boson_core::Run::new(job, "keyset", 1);
        run.run_id = run_id(i);
        run.started_at = keyset_time(base, i);
        b.upsert_run(&run).await.expect("upsert_run");
    }

    let mut scanned: Vec<boson_core::Run> = Vec::new();
    let mut cursor: Option<RunKey> = None;
    let mut pages = 0_usize;
    let newer_id = format!("{prefix}-newer");
    loop {
        let page = b
            .list_runs_after(Some(&job_even), cursor.as_ref(), PAGE)
            .await
            .expect("list_runs_after");
        assert!(page.len() <= PAGE, "page exceeds limit");
        let Some(last) = page.last() else { break };
        cursor = Some(RunKey::from(last));
        scanned.extend(page);
        pages += 1;
        if pages == 2 {
            let mut run = boson_core::Run::new(&job_even, "keyset", 2);
            run.run_id.clone_from(&newer_id);
            b.upsert_run(&run).await.expect("upsert_run mid-scan");
        }
    }

    for pair in scanned.windows(2) {
        assert!(
            RunKey::from(&pair[0]).precedes(&pair[1]),
            "scan out of order: {} then {}",
            pair[0].run_id,
            pair[1].run_id
        );
    }
    assert!(scanned.iter().all(|r| r.job_id == job_even), "job filter");
    let ids: Vec<&str> = scanned.iter().map(|r| r.run_id.as_str()).collect();
    let expected: Vec<String> = (0..TOTAL)
        .rev()
        .filter(|i| i % 2 == 0)
        .map(run_id)
        .collect();
    assert_eq!(
        ids, expected,
        "each run once, newest first; newer run not returned"
    );

    let fresh = b
        .list_runs_after(Some(&job_even), None, 1)
        .await
        .expect("list_runs_after fresh scan");
    assert_eq!(
        fresh.first().map(|r| r.run_id.as_str()),
        Some(newer_id.as_str()),
        "a new scan starts at the newest run"
    );
}
