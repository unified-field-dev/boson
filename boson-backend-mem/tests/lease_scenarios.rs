use boson_backend_mem::MemQueueBackend;
use boson_core::QueueBackend;
use std::sync::Arc;

fn backend() -> Arc<MemQueueBackend> {
    Arc::new(MemQueueBackend::new())
}

#[tokio::test]
async fn lease_contention_second_worker_cannot_claim() {
    let b = backend();
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

#[tokio::test]
async fn extend_lease_refreshes_ttl() {
    let b = backend();
    let lease_id = b
        .try_claim_run_lease("job-2", "worker-a", 60)
        .await
        .unwrap()
        .expect("claim");
    b.extend_lease(&lease_id, 300).await.unwrap();
    assert!(b.expired_lease_job_pairs().await.unwrap().is_empty());
}

#[tokio::test]
async fn expired_lease_pairs_lists_stale_leases() {
    let b = backend();
    let lease_id = b
        .try_claim_run_lease("job-3", "worker-a", -1)
        .await
        .unwrap()
        .expect("claim with negative ttl => already expired");
    let pairs = b.expired_lease_job_pairs().await.unwrap();
    assert!(pairs.iter().any(|(lid, jid)| lid == &lease_id && jid == "job-3"));
}
