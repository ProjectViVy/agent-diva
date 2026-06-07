use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Correlation identifier shared across runtime events for a single trigger.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TraceId(String);

impl TraceId {
    pub fn new() -> Self {
        Self(format!("tr_{}", Uuid::new_v4().simple()))
    }

    pub fn from_raw(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for TraceId {
    fn from(value: String) -> Self {
        Self::from_raw(value)
    }
}

impl From<&str> for TraceId {
    fn from(value: &str) -> Self {
        Self::from_raw(value)
    }
}
