mod common;

use boson_core::{
    default_backend_from_global, JobEnqueueDisposition, JobStatus, QueueBackend, Run, RunStatus,
};
use common::{backend, enqueue, sample_job, task_config, task_config_with_rate_limit};

#[tokio::test]
async fn enqueue_inserts_and_lists() {
    let b = backend();
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    let (job_id, disp) = enqueue(&b, job, &config).await;
    assert_eq!(disp, JobEnqueueDisposition::InsertedNew);
    let listed = b.list_jobs(Some(JobStatus::Queued), 0, 10).await.unwrap();
    assert!(listed.iter().any(|j| j.job_id == job_id));
}

#[tokio::test]
async fn idempotency_reuses_nonterminal() {
    let b = backend();
    let config = task_config("echo");
    let job1 = sample_job("echo", "global", 1, Some("idem-1"));
    let (id1, _) = enqueue(&b, job1, &config).await;
    let job2 = sample_job("echo", "global", 1, Some("idem-1"));
    let (id2, disp) = enqueue(&b, job2, &config).await;
    assert_eq!(disp, JobEnqueueDisposition::ReusedIdempotent);
    assert_eq!(id1, id2);
}

#[tokio::test]
async fn try_claim_atomic() {
    let b = backend();
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    let (job_id, _) = enqueue(&b, job, &config).await;
    assert!(b.try_claim_job(&job_id).await.unwrap().is_some());
    assert!(b.try_claim_job(&job_id).await.unwrap().is_none());
}

#[tokio::test]
async fn pool_priority_order() {
    let b = backend();
    let config = task_config("echo");
    let low = sample_job("echo", "workers", 1, None);
    let high = sample_job("echo", "workers", 5, None);
    enqueue(&b, high, &config).await;
    enqueue(&b, low, &config).await;
    let queued = b.list_queued_for_pool_sorted("workers", 10).await.unwrap();
    assert_eq!(queued.len(), 2);
    assert!(queued[0].priority <= queued[1].priority);
}

#[tokio::test]
async fn max_in_flight_rate_limit() {
    let b = backend();
    let config = task_config_with_rate_limit("echo", 1, 0);
    let job1 = sample_job("echo", "global", 1, None);
    enqueue(&b, job1, &config).await;
    let job2 = sample_job("echo", "global", 1, None);
    let err = b
        .enqueue_with_policies(job2, &config)
        .await
        .unwrap_err();
    assert!(matches!(err, boson_core::BosonError::RateLimited(_)));
}

#[tokio::test]
async fn max_enqueue_per_second() {
    let b = backend();
    let config = task_config_with_rate_limit("echo", 0, 1);
    let job1 = sample_job("echo", "global", 1, None);
    enqueue(&b, job1, &config).await;
    let job2 = sample_job("echo", "global", 1, None);
    let err = b
        .enqueue_with_policies(job2, &config)
        .await
        .unwrap_err();
    assert!(matches!(err, boson_core::BosonError::RateLimited(_)));
}

#[tokio::test]
async fn worker_scenario_happy_path() {
    let b = backend();
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    let (job_id, _) = enqueue(&b, job, &config).await;

    let queued = b.list_queued_for_pool_sorted("global", 1).await.unwrap();
    assert_eq!(queued[0].job_id, job_id);

    let claimed = b.try_claim_job(&job_id).await.unwrap().expect("claimed");
    assert_eq!(claimed.status, JobStatus::Running);

    let run = Run::new(&job_id, "echo", 1);
    let run_id = run.run_id.clone();
    b.upsert_run(&run).await.unwrap();
    b.finish_run(&run_id, RunStatus::Success, Some(42), None)
        .await
        .unwrap();

    let finished = b.get_run(&run_id).await.unwrap().expect("run");
    assert_eq!(finished.status, RunStatus::Success);
    assert_eq!(finished.duration_ms, Some(42));
}

#[tokio::test]
async fn register_on_router() {
    let _ = boson_backend_mem::install_default_mem_backend();
    let resolved = default_backend_from_global().expect("default backend");
    let config = task_config("echo");
    let job = sample_job("echo", "global", 1, None);
    let (job_id, _) = resolved
        .enqueue_with_policies(job, &config)
        .await
        .expect("enqueue via global");
    assert!(!job_id.is_empty());
}
