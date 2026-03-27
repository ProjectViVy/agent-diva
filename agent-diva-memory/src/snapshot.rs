//! Markdown snapshot export and hydrate helpers for disaster recovery.

use crate::layout::snapshot_path;
use crate::types::MemoryRecord;
use agent_diva_core::{Error, Result};
use std::fs;
use std::path::Path;

const SNAPSHOT_HEADER: &str = "# Agent Diva Memory Snapshot\n\n\
This file is auto-generated from the enhanced memory store and can be used to hydrate a fresh `brain.db`.\n";

pub fn export_snapshot<P: AsRef<Path>>(workspace: P, records: &[MemoryRecord]) -> Result<usize> {
    let workspace = workspace.as_ref();
    let path = snapshot_path(workspace);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut output = String::from(SNAPSHOT_HEADER);
    output.push_str(&format!("\nTotal records: {}\n", records.len()));
    for record in records {
        output.push_str("\n---\n\n");
        output.push_str(&format!("## Record `{}`\n\n", record.id));
        output.push_str("```json\n");
        output.push_str(&serde_json::to_string_pretty(record).map_err(|error| {
            Error::Internal(format!("failed to serialize snapshot record: {error}"))
        })?);
        output.push_str("\n```\n");
    }

    fs::write(path, output)?;
    Ok(records.len())
}

pub fn hydrate_snapshot<P: AsRef<Path>>(workspace: P) -> Result<Vec<MemoryRecord>> {
    let path = snapshot_path(workspace.as_ref());
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path)?;
    let mut records = Vec::new();
    let mut in_json = false;
    let mut current = String::new();

    for line in content.lines() {
        if line.trim() == "```json" {
            in_json = true;
            current.clear();
            continue;
        }
        if line.trim() == "```" && in_json {
            in_json = false;
            if !current.trim().is_empty() {
                let record = serde_json::from_str::<MemoryRecord>(&current).map_err(|error| {
                    Error::Internal(format!("failed to parse snapshot record: {error}"))
                })?;
                records.push(record);
            }
            continue;
        }
        if in_json {
            current.push_str(line);
            current.push('\n');
        }
    }

    Ok(records)
}

pub fn snapshot_exists<P: AsRef<Path>>(workspace: P) -> bool {
    snapshot_path(workspace.as_ref()).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryDomain, MemoryScope, MemorySourceRef};
    use chrono::{DateTime, Utc};
    use tempfile::TempDir;

    fn sample_record() -> MemoryRecord {
        MemoryRecord {
            id: "compat-memory-md-1".into(),
            timestamp: DateTime::parse_from_rfc3339("2026-03-27T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            domain: MemoryDomain::Workspace,
            scope: MemoryScope::Workspace,
            title: "Snapshot".into(),
            summary: "Persist state".into(),
            content: "Persist state from diary and MEMORY chunks.".into(),
            tags: vec!["snapshot".into()],
            source_refs: vec![MemorySourceRef {
                path: Some("memory/MEMORY.md".into()),
                section: None,
                note: None,
            }],
            confidence: 0.8,
        }
    }

    #[test]
    fn snapshot_roundtrip_preserves_records() {
        let temp = TempDir::new().unwrap();
        let records = vec![sample_record()];
        export_snapshot(temp.path(), &records).unwrap();
        let hydrated = hydrate_snapshot(temp.path()).unwrap();
        assert_eq!(hydrated, records);
    }
}
