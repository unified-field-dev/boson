//! Execution mode for the Boson work engine.

use serde::{Deserialize, Serialize};

/// Execution mode for the Boson work engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BosonMode {
    /// In-memory queue and worker pools.
    #[default]
    Local,

    /// Shared queue with leases across worker replicas.
    Distributed,

    /// Coordinator calls remote HTTP API; no in-process worker.
    Remote,
}

impl BosonMode {
    /// Parse mode from environment variable `BOSON_MODE`.
    ///
    /// Returns [`Local`](Self::Local) if not set or invalid.
    pub fn from_env() -> Self {
        std::env::var("BOSON_MODE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default()
    }
}

impl std::str::FromStr for BosonMode {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "local" => Ok(BosonMode::Local),
            "distributed" => Ok(BosonMode::Distributed),
            "remote" => Ok(BosonMode::Remote),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for BosonMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BosonMode::Local => write!(f, "local"),
            BosonMode::Distributed => write!(f, "distributed"),
            BosonMode::Remote => write!(f, "remote"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_from_str() {
        assert_eq!("local".parse::<BosonMode>().unwrap(), BosonMode::Local);
        assert_eq!("LOCAL".parse::<BosonMode>().unwrap(), BosonMode::Local);
        assert_eq!(
            "distributed".parse::<BosonMode>().unwrap(),
            BosonMode::Distributed
        );
        assert!("unknown".parse::<BosonMode>().is_err());
    }

    #[test]
    fn mode_display() {
        assert_eq!(BosonMode::Local.to_string(), "local");
        assert_eq!(BosonMode::Distributed.to_string(), "distributed");
        assert_eq!(BosonMode::Remote.to_string(), "remote");
    }
}
