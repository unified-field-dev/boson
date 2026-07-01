//! Task config persistence helpers for the in-memory backend.

use boson_core::{Result, TaskConfig};

use crate::store::Inner;

/// Load task config by name.
pub fn get_task_config(inner: &Inner, task_name: &str) -> Result<Option<TaskConfig>> {
    Ok(inner.task_configs.get(task_name).cloned())
}

/// Persist task config.
pub fn upsert_task_config(inner: &mut Inner, config: &TaskConfig) -> Result<()> {
    inner
        .task_configs
        .insert(config.task_name.clone(), config.clone());
    Ok(())
}
