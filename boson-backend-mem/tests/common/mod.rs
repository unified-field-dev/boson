//! Shared helpers for integration tests.

use boson_backend_mem::MemQueueBackend;
use boson_core::{Job, QueueBackend, RateLimitPolicy, TaskConfig};
use serde_json::json;
use std::sync::Arc;

pub fn backend() -> Arc<MemQueueBackend> {
    Arc::new(MemQueueBackend::new())
}

pub fn task_config(task_name: &str) -> TaskConfig {
    TaskConfig::default_for(task_name)
}

pub fn task_config_with_rate_limit(task_name: &str, max_in_flight: u32, max_eps: u32) -> TaskConfig {
    let mut config = TaskConfig::default_for(task_name);
    config.rate_limit_policy = RateLimitPolicy {
        max_in_flight,
        max_enqueue_per_second: max_eps,
    };
    config
}

pub fn sample_job(task_name: &str, pool: &str, priority: i32, idempotency_key: Option<&str>) -> Job {
    Job::new(
        task_name,
        json!({"label": "test"}),
        json!({}),
        priority,
        pool,
        0,
        idempotency_key.map(str::to_string),
    )
}

pub async fn enqueue(
    backend: &MemQueueBackend,
    job: Job,
    config: &TaskConfig,
) -> (String, boson_core::JobEnqueueDisposition) {
    backend
        .enqueue_with_policies(job, config)
        .await
        .expect("enqueue")
}