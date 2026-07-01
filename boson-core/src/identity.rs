//! Identity reconstruction port (host adapter maps JSON to app identity).

use serde_json::Value;

use crate::error::IdentityError;

/// Opaque execution context for task handlers (host adapter maps to app identity).
pub trait ExecutionContext: Send {
    /// Debug label for logs and tests.
    fn label(&self) -> &str;
}

/// Builds handler execution context from captured actor JSON at enqueue time.
pub trait ExecutionContextFactory: Send + Sync {
    /// Build context for task dispatch from stored `actor_json`.
    fn build(&self, actor_json: &Value) -> Result<Box<dyn ExecutionContext>, IdentityError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestContext {
        label: String,
    }

    impl ExecutionContext for TestContext {
        fn label(&self) -> &str {
            &self.label
        }
    }

    struct TestFactory;

    impl ExecutionContextFactory for TestFactory {
        fn build(&self, actor_json: &Value) -> Result<Box<dyn ExecutionContext>, IdentityError> {
            if actor_json.get("System").is_some() {
                Ok(Box::new(TestContext {
                    label: "system".into(),
                }))
            } else {
                Err(IdentityError::InvalidActor("missing System".into()))
            }
        }
    }

    #[test]
    fn factory_builds_context() {
        let factory = TestFactory;
        let ctx = factory
            .build(&serde_json::json!({"System": {"operation": "t"}}))
            .expect("ok");
        assert_eq!(ctx.label(), "system");
    }
}
