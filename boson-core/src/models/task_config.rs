//! Task config model — priority, pool, retry, and rate limits.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Retry policy for a task.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Max retry attempts.
    pub max_attempts: u32,
    /// Base delay in ms.
    pub base_delay_ms: u64,
    /// Backoff multiplier.
    pub backoff_multiplier: f64,
    /// Max delay in ms.
    pub max_delay_ms: u64,
}

/// Rate limiting for enqueue backpressure.
///
/// `max_in_flight == 0` and `max_enqueue_per_second == 0` mean unlimited (default).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitPolicy {
    /// Max jobs for this task in `queued` + `running` at once. `0` = no limit.
    pub max_in_flight: u32,
    /// Max successful enqueues per wall-clock second per process. `0` = no limit.
    pub max_enqueue_per_second: u32,
}

/// Per-task config persisted for admin UI and enqueue defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    /// Task name (unique key).
    pub task_name: String,
    /// Override priority (lower = higher priority).
    pub priority: i32,
    /// Override pool name.
    pub pool: String,
    /// Retry policy override.
    pub retry_policy: RetryPolicy,
    /// Enqueue rate limits (optional).
    pub rate_limit_policy: RateLimitPolicy,
    /// When created/updated.
    pub updated_at: DateTime<Utc>,
}

impl TaskConfig {
    /// Create default config for a task name.
    pub fn default_for(task_name: &str) -> Self {
        Self {
            task_name: task_name.to_string(),
            priority: 1,
            pool: "global".to_string(),
            retry_policy: RetryPolicy::default(),
            rate_limit_policy: RateLimitPolicy::default(),
            updated_at: Utc::now(),
        }
    }
}
