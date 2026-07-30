//! Payload-free execution evidence for AutoDream reflection.

use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_RETENTION: usize = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    Succeeded,
    Failed,
    Denied,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationState {
    ExecutorObserved,
    SessionBackfill,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceEvidence {
    pub schema_version: u32,
    pub id: String,
    pub digest: String,
    pub workspace_id: String,
    pub session_id: String,
    pub trace_id: String,
    pub action_id: String,
    pub tool_category: String,
    pub outcome: OutcomeKind,
    pub verification: VerificationState,
    pub summary: String,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceBatch {
    pub workspace_id: String,
    pub items: Vec<ExperienceEvidence>,
    pub rejected_lines: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone)]
pub struct ExperienceJournal {
    workspace_root: PathBuf,
    journal_path: PathBuf,
    retention: usize,
}

impl ExperienceJournal {
    pub fn open(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let journal_path = workspace_root
            .join(".agent-diva")
            .join("autodream")
            .join("experience")
            .join("events.jsonl");
        Self {
            workspace_root,
            journal_path,
            retention: DEFAULT_RETENTION,
        }
    }

    pub fn with_retention(mut self, retention: usize) -> Self {
        self.retention = retention.max(1);
        self
    }

    pub fn workspace_id(&self) -> String {
        workspace_digest(&self.workspace_root)
    }

    pub fn path(&self) -> &Path {
        &self.journal_path
    }

    pub fn tool_evidence(
        &self,
        session_id: &str,
        trace_id: &str,
        action_id: &str,
        tool_name: &str,
        outcome: OutcomeKind,
    ) -> ExperienceEvidence {
        let workspace_id = self.workspace_id();
        let tool_category = classify_tool(tool_name).to_string();
        let summary = format!(
            "{} action {}",
            tool_category,
            match outcome {
                OutcomeKind::Succeeded => "succeeded",
                OutcomeKind::Failed => "failed",
                OutcomeKind::Denied => "was denied",
                OutcomeKind::Cancelled => "was cancelled",
            }
        );
        let verification = VerificationState::ExecutorObserved;
        let material = format!(
            "{workspace_id}\0{session_id}\0{trace_id}\0{action_id}\0{tool_category}\0{:?}\0{:?}",
            outcome, verification
        );
        let digest = sha256(material.as_bytes());
        ExperienceEvidence {
            schema_version: SCHEMA_VERSION,
            id: format!("exp-{}", &digest[..32]),
            digest,
            workspace_id,
            session_id: session_id.to_string(),
            trace_id: trace_id.to_string(),
            action_id: action_id.to_string(),
            tool_category,
            outcome,
            verification,
            summary,
            occurred_at: Utc::now(),
        }
    }

    /// Append once by deterministic evidence id. No tool arguments, output, or Memory payload
    /// crosses this boundary.
    pub fn append(&self, evidence: &ExperienceEvidence) -> std::io::Result<bool> {
        if evidence.workspace_id != self.workspace_id() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "experience workspace identity mismatch",
            ));
        }
        let parent = self
            .journal_path
            .parent()
            .expect("experience journal always has a parent");
        fs::create_dir_all(parent)?;
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&self.journal_path)?;
        file.lock_exclusive()?;

        let existing = read_locked(&file, &evidence.workspace_id, usize::MAX)?;
        if existing.items.iter().any(|item| item.id == evidence.id) {
            FileExt::unlock(&file)?;
            return Ok(false);
        }

        if existing.items.len() >= self.retention {
            file.set_len(0)?;
            file.seek(SeekFrom::Start(0))?;
            for retained in existing
                .items
                .iter()
                .skip(existing.items.len() + 1 - self.retention)
            {
                serde_json::to_writer(&mut file, retained)?;
                file.write_all(b"\n")?;
            }
        } else {
            file.seek(SeekFrom::End(0))?;
        }
        serde_json::to_writer(&mut file, evidence)?;
        file.write_all(b"\n")?;
        file.flush()?;
        file.sync_data()?;
        FileExt::unlock(&file)?;
        Ok(true)
    }

    pub fn read_recent(&self, limit: usize) -> std::io::Result<ExperienceBatch> {
        let workspace_id = self.workspace_id();
        if !self.journal_path.exists() {
            return Ok(ExperienceBatch {
                workspace_id,
                items: Vec::new(),
                rejected_lines: 0,
                truncated: false,
            });
        }
        let file = File::open(&self.journal_path)?;
        FileExt::lock_shared(&file)?;
        let result = read_locked(&file, &workspace_id, limit.min(self.retention));
        FileExt::unlock(&file)?;
        result
    }
}

fn read_locked(file: &File, workspace_id: &str, limit: usize) -> std::io::Result<ExperienceBatch> {
    let mut items = Vec::new();
    let mut rejected_lines = 0;
    let mut seen = HashSet::new();
    for line in BufReader::new(file.try_clone()?).lines() {
        let line = line?;
        match serde_json::from_str::<ExperienceEvidence>(&line) {
            Ok(item)
                if item.schema_version == SCHEMA_VERSION
                    && item.workspace_id == workspace_id
                    && seen.insert(item.id.clone()) =>
            {
                items.push(item);
            }
            _ => rejected_lines += 1,
        }
    }
    let truncated = items.len() > limit;
    if truncated {
        items.drain(0..items.len() - limit);
    }
    Ok(ExperienceBatch {
        workspace_id: workspace_id.to_string(),
        items,
        rejected_lines,
        truncated,
    })
}

fn workspace_digest(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    format!(
        "workspace-{}",
        &sha256(canonical.to_string_lossy().as_bytes())[..32]
    )
}

fn sha256(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn classify_tool(name: &str) -> &'static str {
    match name {
        "exec" => "command",
        "read_file" | "list_dir" | "read_attachment" => "read",
        "write_file" | "edit_file" | "apply_patch" => "filesystem_mutation",
        "web_search" | "web_fetch" => "web",
        "cron" => "schedule",
        "spawn" | "spawn_agent" => "delegation",
        name if name.starts_with("plan_") || name.starts_with("todo_") => "planning",
        _ => "tool",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_is_idempotent_and_never_persists_payload() {
        let temp = tempfile::tempdir().unwrap();
        let journal = ExperienceJournal::open(temp.path());
        let evidence = journal.tool_evidence(
            "session-1",
            "trace-1",
            "call-1",
            "exec",
            OutcomeKind::Succeeded,
        );

        assert!(journal.append(&evidence).unwrap());
        assert!(!journal.append(&evidence).unwrap());
        let raw = fs::read_to_string(journal.path()).unwrap();
        assert!(!raw.contains("secret"));
        assert!(!raw.contains("arguments"));
        assert_eq!(journal.read_recent(10).unwrap().items, vec![evidence]);
    }

    #[test]
    fn rejects_cross_workspace_evidence_and_corrupt_lines() {
        let left = tempfile::tempdir().unwrap();
        let right = tempfile::tempdir().unwrap();
        let left_journal = ExperienceJournal::open(left.path());
        let evidence = ExperienceJournal::open(right.path()).tool_evidence(
            "s",
            "t",
            "a",
            "exec",
            OutcomeKind::Failed,
        );
        assert_eq!(
            left_journal.append(&evidence).unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );

        fs::create_dir_all(left_journal.path().parent().unwrap()).unwrap();
        fs::write(left_journal.path(), "{bad json}\n").unwrap();
        let batch = left_journal.read_recent(10).unwrap();
        assert_eq!(batch.rejected_lines, 1);
        assert!(batch.items.is_empty());
    }

    #[test]
    fn retention_returns_only_newest_items() {
        let temp = tempfile::tempdir().unwrap();
        let journal = ExperienceJournal::open(temp.path()).with_retention(2);
        for action in ["a", "b", "c"] {
            let item = journal.tool_evidence("s", "t", action, "read_file", OutcomeKind::Succeeded);
            journal.append(&item).unwrap();
        }
        let batch = journal.read_recent(10).unwrap();
        assert!(!batch.truncated);
        assert_eq!(batch.items.len(), 2);
        assert_eq!(batch.items[0].action_id, "b");
        assert_eq!(
            fs::read_to_string(journal.path()).unwrap().lines().count(),
            2
        );
    }
}
