//! Cron types for scheduled jobs

use serde::{Deserialize, Serialize};

/// Schedule definition for a cron job
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CronSchedule {
    /// Run at a specific timestamp
    #[serde(rename = "at")]
    At {
        #[serde(rename = "atMs")]
        at_ms: i64,
    },
    /// Run at regular intervals
    #[serde(rename = "every")]
    Every {
        #[serde(rename = "everyMs")]
        every_ms: i64,
    },
    /// Run on cron expression
    #[serde(rename = "cron")]
    Cron {
        expr: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tz: Option<String>,
    },
}

impl CronSchedule {
    /// Create a one-time schedule
    pub fn at(at_ms: i64) -> Self {
        Self::At { at_ms }
    }

    /// Create a recurring schedule
    pub fn every(every_ms: i64) -> Self {
        Self::Every { every_ms }
    }

    /// Create a cron expression schedule
    pub fn cron(expr: String, tz: Option<String>) -> Self {
        Self::Cron { expr, tz }
    }
}

/// What to do when the job runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronPayload {
    #[serde(default = "default_payload_kind")]
    pub kind: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub deliver: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
}

fn default_payload_kind() -> String {
    "agent_turn".to_string()
}

impl Default for CronPayload {
    fn default() -> Self {
        Self {
            kind: "agent_turn".to_string(),
            message: String::new(),
            deliver: false,
            channel: None,
            to: None,
        }
    }
}

/// Runtime state of a job
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CronJobState {
    #[serde(rename = "nextRunAtMs", skip_serializing_if = "Option::is_none")]
    pub next_run_at_ms: Option<i64>,
    #[serde(rename = "lastRunAtMs", skip_serializing_if = "Option::is_none")]
    pub last_run_at_ms: Option<i64>,
    #[serde(rename = "lastStatus", skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(rename = "lastError", skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// A scheduled job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub schedule: CronSchedule,
    #[serde(default)]
    pub payload: CronPayload,
    #[serde(default)]
    pub state: CronJobState,
    #[serde(rename = "createdAtMs", default)]
    pub created_at_ms: i64,
    #[serde(rename = "updatedAtMs", default)]
    pub updated_at_ms: i64,
    #[serde(rename = "deleteAfterRun", default)]
    pub delete_after_run: bool,
}

fn default_true() -> bool {
    true
}

/// Persistent store for cron jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronStore {
    #[serde(default = "default_version")]
    pub version: i32,
    #[serde(default)]
    pub jobs: Vec<CronJob>,
}

fn default_version() -> i32 {
    1
}

impl Default for CronStore {
    fn default() -> Self {
        Self {
            version: 1,
            jobs: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_schedule_at() {
        let schedule = CronSchedule::at(1000);
        assert!(matches!(schedule, CronSchedule::At { at_ms: 1000 }));
    }

    #[test]
    fn test_cron_schedule_every() {
        let schedule = CronSchedule::every(5000);
        assert!(matches!(schedule, CronSchedule::Every { every_ms: 5000 }));
    }

    #[test]
    fn test_cron_schedule_cron() {
        let schedule = CronSchedule::cron("0 9 * * *".to_string(), None);
        assert!(matches!(schedule, CronSchedule::Cron { .. }));
    }

    #[test]
    fn test_cron_payload_default() {
        let payload = CronPayload::default();
        assert_eq!(payload.kind, "agent_turn");
        assert_eq!(payload.message, "");
        assert!(!payload.deliver);
        assert!(payload.channel.is_none());
    }

    #[test]
    fn test_cron_job_state_default() {
        let state = CronJobState::default();
        assert!(state.next_run_at_ms.is_none());
        assert!(state.last_run_at_ms.is_none());
        assert!(state.last_status.is_none());
    }

    #[test]
    fn test_cron_store_default() {
        let store = CronStore::default();
        assert_eq!(store.version, 1);
        assert!(store.jobs.is_empty());
    }

    #[test]
    fn test_cron_schedule_serialization() {
        let schedule = CronSchedule::every(5000);
        let json = serde_json::to_string(&schedule).unwrap();
        assert!(json.contains("\"kind\":\"every\""));
        assert!(json.contains("\"everyMs\":5000"));

        let deserialized: CronSchedule = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            deserialized,
            CronSchedule::Every { every_ms: 5000 }
        ));
    }
}
