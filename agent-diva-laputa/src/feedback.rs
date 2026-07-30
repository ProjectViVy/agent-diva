use std::{fs, time::Duration};

use agent_diva_core::governance::ContentDigest;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{atomic_write_json, LaputaLock, LaputaStorage, LockOptions, Result};

const SCHEMA_VERSION: u32 = 1;
const MAX_FEEDBACK_EVENTS: usize = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecallTaskOutcome {
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecallFeedbackEvent {
    pub schema_version: u32,
    pub event_id: String,
    pub request_id: String,
    pub record_id: String,
    pub content_digest: ContentDigest,
    pub selected: bool,
    pub injected: bool,
    pub corrected: bool,
    pub task_outcome: RecallTaskOutcome,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRecallFeedback {
    pub request_id: String,
    pub selected: Vec<(String, ContentDigest)>,
    pub injected: bool,
    pub selected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecallFeedbackFile {
    schema_version: u32,
    events: Vec<RecallFeedbackEvent>,
}

impl Default for RecallFeedbackFile {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            events: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecallFeedbackStore {
    storage: LaputaStorage,
}

impl RecallFeedbackStore {
    pub fn new(storage: LaputaStorage) -> Self {
        Self { storage }
    }

    pub fn commit_pending(
        &self,
        pending: Vec<PendingRecallFeedback>,
        task_outcome: RecallTaskOutcome,
        corrected: bool,
        recorded_at: DateTime<Utc>,
    ) -> Result<Vec<RecallFeedbackEvent>> {
        if pending.is_empty() {
            return Ok(Vec::new());
        }
        let _lock = LaputaLock::acquire(
            self.storage.paths().lock_file("recall-feedback"),
            LockOptions {
                timeout: Duration::from_secs(5),
                ..LockOptions::default()
            },
        )?;
        let mut file = self.read_file()?;
        let mut committed = Vec::new();
        for recall in pending {
            for (record_id, content_digest) in recall.selected {
                let event = RecallFeedbackEvent {
                    schema_version: SCHEMA_VERSION,
                    event_id: format!(
                        "recall-feedback:{}:{}:{}",
                        recall.request_id,
                        record_id,
                        recall.selected_at.timestamp_micros()
                    ),
                    request_id: recall.request_id.clone(),
                    record_id,
                    content_digest,
                    selected: true,
                    injected: recall.injected,
                    corrected,
                    task_outcome,
                    recorded_at,
                };
                if !file
                    .events
                    .iter()
                    .any(|existing| existing.event_id == event.event_id)
                {
                    file.events.push(event.clone());
                }
                committed.push(event);
            }
        }
        file.events
            .sort_by(|left, right| left.recorded_at.cmp(&right.recorded_at));
        if file.events.len() > MAX_FEEDBACK_EVENTS {
            file.events.drain(..file.events.len() - MAX_FEEDBACK_EVENTS);
        }
        atomic_write_json(self.storage.paths().recall_feedback_json(), &file)?;
        Ok(committed)
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<RecallFeedbackEvent>> {
        let mut events = self.read_file()?.events;
        events.sort_by(|left, right| {
            right
                .recorded_at
                .cmp(&left.recorded_at)
                .then_with(|| left.event_id.cmp(&right.event_id))
        });
        events.truncate(limit.min(MAX_FEEDBACK_EVENTS));
        Ok(events)
    }

    fn read_file(&self) -> Result<RecallFeedbackFile> {
        let path = self.storage.paths().recall_feedback_json();
        if !path.exists() {
            return Ok(RecallFeedbackFile::default());
        }
        let bytes = fs::read(&path).map_err(|source| crate::LaputaError::io(&path, source))?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}
