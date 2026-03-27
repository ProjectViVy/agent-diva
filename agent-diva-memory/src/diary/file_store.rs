//! File-backed diary storage for Phase A rational diary persistence.

use crate::contracts::DiaryStore;
use crate::layout::diary_dir_path;
use crate::types::{
    DiaryEntry, DiaryFilter, DiaryPartition, MemoryDomain, MemoryScope, MemorySourceRef,
};
use chrono::{DateTime, Local, Utc};
use std::path::{Path, PathBuf};

const ENTRY_MARKER_PREFIX: &str = "<!-- agent-diva:diary-entry ";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DiaryEntryEnvelope {
    id: String,
    timestamp: DateTime<Utc>,
    partition: DiaryPartition,
    domain: MemoryDomain,
    scope: MemoryScope,
    title: String,
    summary: String,
    body: String,
    tags: Vec<String>,
    source_refs: Vec<MemorySourceRef>,
    confidence: f32,
    observations: Vec<String>,
    confirmed: Vec<String>,
    unknowns: Vec<String>,
    next_steps: Vec<String>,
}

impl From<&DiaryEntry> for DiaryEntryEnvelope {
    fn from(entry: &DiaryEntry) -> Self {
        Self {
            id: entry.id.clone(),
            timestamp: entry.timestamp,
            partition: entry.partition.clone(),
            domain: entry.domain.clone(),
            scope: entry.scope.clone(),
            title: entry.title.clone(),
            summary: entry.summary.clone(),
            body: entry.body.clone(),
            tags: entry.tags.clone(),
            source_refs: entry.source_refs.clone(),
            confidence: entry.confidence,
            observations: entry.observations.clone(),
            confirmed: entry.confirmed.clone(),
            unknowns: entry.unknowns.clone(),
            next_steps: entry.next_steps.clone(),
        }
    }
}

impl From<DiaryEntryEnvelope> for DiaryEntry {
    fn from(value: DiaryEntryEnvelope) -> Self {
        Self {
            id: value.id,
            timestamp: value.timestamp,
            partition: value.partition,
            domain: value.domain,
            scope: value.scope,
            title: value.title,
            summary: value.summary,
            body: value.body,
            tags: value.tags,
            source_refs: value.source_refs,
            confidence: value.confidence,
            observations: value.observations,
            confirmed: value.confirmed,
            unknowns: value.unknowns,
            next_steps: value.next_steps,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileDiaryStore {
    workspace: PathBuf,
}

impl FileDiaryStore {
    pub fn new<P: AsRef<Path>>(workspace: P) -> Self {
        Self {
            workspace: workspace.as_ref().to_path_buf(),
        }
    }

    pub fn diary_root(&self) -> PathBuf {
        diary_dir_path(&self.workspace)
    }

    pub fn partition_dir(&self, partition: &DiaryPartition) -> PathBuf {
        match partition {
            DiaryPartition::Rational => self.diary_root().join("rational"),
            DiaryPartition::Emotional => self.diary_root().join("emotional"),
        }
    }

    fn ensure_partition_dir(&self, partition: &DiaryPartition) -> agent_diva_core::Result<PathBuf> {
        let dir = self.partition_dir(partition);
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn file_path_for_day(&self, date: &str, partition: &DiaryPartition) -> PathBuf {
        self.partition_dir(partition).join(format!("{date}.md"))
    }

    fn render_day_header(partition: &DiaryPartition, date: &str) -> String {
        let partition_label = match partition {
            DiaryPartition::Rational => "Rational",
            DiaryPartition::Emotional => "Emotional",
        };
        format!("# {partition_label} Diary - {date}\n\n")
    }

    fn render_entry(entry: &DiaryEntry) -> agent_diva_core::Result<String> {
        let envelope = DiaryEntryEnvelope::from(entry);
        let metadata = serde_json::to_string(&envelope)?;
        let mut output = String::new();
        output.push_str(ENTRY_MARKER_PREFIX);
        output.push_str(&metadata);
        output.push_str(" -->\n");
        output.push_str("## ");
        output.push_str(&entry.title);
        output.push('\n');
        output.push_str("*Timestamp:* ");
        output.push_str(&entry.timestamp.to_rfc3339());
        output.push_str("  \n");
        output.push_str("*Domain:* `");
        output.push_str(&serde_json::to_string(&entry.domain)?.replace('"', ""));
        output.push_str("`  \n");
        output.push_str("*Scope:* `");
        output.push_str(&serde_json::to_string(&entry.scope)?.replace('"', ""));
        output.push_str("`  \n");
        output.push_str("*Confidence:* ");
        output.push_str(&format!("{:.2}", entry.confidence));
        output.push_str("\n\n### Summary\n");
        output.push_str(entry.summary.trim());
        output.push_str("\n\n### Observations\n");
        output.push_str(&render_list(&entry.observations));
        output.push_str("\n\n### Confirmed\n");
        output.push_str(&render_list(&entry.confirmed));
        output.push_str("\n\n### Unknowns\n");
        output.push_str(&render_list(&entry.unknowns));
        output.push_str("\n\n### Next Steps\n");
        output.push_str(&render_list(&entry.next_steps));
        output.push_str("\n\n### Sources\n");
        output.push_str(&render_sources(&entry.source_refs));
        output.push_str("\n\n### Body\n");
        output.push_str(entry.body.trim());
        output.push_str("\n\n");
        Ok(output)
    }

    fn parse_entries(content: &str) -> Vec<DiaryEntry> {
        let mut entries = Vec::new();
        for chunk in content.split(ENTRY_MARKER_PREFIX).skip(1) {
            let Some((meta, _rest)) = chunk.split_once(" -->\n") else {
                continue;
            };
            match serde_json::from_str::<DiaryEntryEnvelope>(meta) {
                Ok(parsed) => entries.push(parsed.into()),
                Err(error) => tracing::warn!("Failed to parse diary entry metadata: {}", error),
            }
        }
        entries
    }

    fn entry_matches_filter(entry: &DiaryEntry, filter: &DiaryFilter) -> bool {
        if let Some(partition) = &filter.partition {
            if entry.partition != *partition {
                return false;
            }
        }
        if let Some(domain) = &filter.domain {
            if entry.domain != *domain {
                return false;
            }
        }
        if let Some(scope) = &filter.scope {
            if entry.scope != *scope {
                return false;
            }
        }
        if let Some(since) = filter.since {
            if entry.timestamp < since {
                return false;
            }
        }
        if let Some(until) = filter.until {
            if entry.timestamp > until {
                return false;
            }
        }
        if let Some(tag) = &filter.tag {
            if !entry.tags.iter().any(|value| value == tag) {
                return false;
            }
        }
        true
    }
}

impl DiaryStore for FileDiaryStore {
    fn append_entry(&self, entry: &DiaryEntry) -> agent_diva_core::Result<()> {
        let dir = self.ensure_partition_dir(&entry.partition)?;
        let date = entry.timestamp.with_timezone(&Local).format("%Y-%m-%d");
        let path = dir.join(format!("{date}.md"));

        let mut content = if path.exists() {
            std::fs::read_to_string(&path)?
        } else {
            Self::render_day_header(&entry.partition, &date.to_string())
        };

        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&Self::render_entry(entry)?);
        std::fs::write(path, content)?;
        Ok(())
    }

    fn load_day(
        &self,
        date: &str,
        partition: DiaryPartition,
    ) -> agent_diva_core::Result<Vec<DiaryEntry>> {
        let path = self.file_path_for_day(date, &partition);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(path)?;
        Ok(Self::parse_entries(&content))
    }

    fn list_days(&self, partition: DiaryPartition) -> agent_diva_core::Result<Vec<String>> {
        let dir = self.partition_dir(&partition);
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut days = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                if name.len() == 13 && name.ends_with(".md") {
                    days.push(name.trim_end_matches(".md").to_string());
                }
            }
        }
        days.sort_by(|left, right| right.cmp(left));
        Ok(days)
    }

    fn filter_entries(&self, filter: &DiaryFilter) -> agent_diva_core::Result<Vec<DiaryEntry>> {
        let partition = filter.partition.clone().unwrap_or(DiaryPartition::Rational);
        let days = self.list_days(partition.clone())?;
        let mut entries = Vec::new();

        for day in days {
            for entry in self.load_day(&day, partition.clone())? {
                if Self::entry_matches_filter(&entry, filter) {
                    entries.push(entry);
                }
            }
        }

        entries.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        if let Some(limit) = filter.limit {
            entries.truncate(limit);
        }
        Ok(entries)
    }
}

fn render_list(items: &[String]) -> String {
    if items.is_empty() {
        "- (none)".into()
    } else {
        items
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn render_sources(sources: &[MemorySourceRef]) -> String {
    if sources.is_empty() {
        return "- (none)".into();
    }

    sources
        .iter()
        .map(|source| {
            let mut parts = Vec::new();
            if let Some(path) = &source.path {
                parts.push(format!("`{path}`"));
            }
            if let Some(section) = &source.section {
                parts.push(section.clone());
            }
            if let Some(note) = &source.note {
                parts.push(note.clone());
            }
            format!("- {}", parts.join(" | "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_entry() -> DiaryEntry {
        let mut entry = DiaryEntry::new(
            DiaryPartition::Rational,
            MemoryDomain::Workspace,
            MemoryScope::Workspace,
            "Workspace memory design",
            "Summarized the workspace memory layout.",
            "Detailed body",
        );
        entry.timestamp = DateTime::parse_from_rfc3339("2026-03-26T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        entry.tags = vec!["workspace".into(), "design".into()];
        entry.source_refs = vec![MemorySourceRef {
            path: Some("agent-diva-core/src/memory/mod.rs".into()),
            section: Some("memory".into()),
            note: Some("module exports".into()),
        }];
        entry.observations = vec!["Found a lightweight MEMORY.md pipeline.".into()];
        entry.confirmed = vec!["ContextBuilder only injects MEMORY.md.".into()];
        entry.unknowns = vec!["Recall engine not implemented.".into()];
        entry.next_steps = vec!["Add diary store.".into()];
        entry.confidence = 0.82;
        entry
    }

    #[test]
    fn test_append_and_load_rational_diary_entries() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileDiaryStore::new(temp_dir.path());
        let entry = sample_entry();

        store.append_entry(&entry).unwrap();

        let loaded = store
            .load_day("2026-03-26", DiaryPartition::Rational)
            .unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].title, "Workspace memory design");
        assert_eq!(loaded[0].tags, vec!["workspace", "design"]);
    }

    #[test]
    fn test_filter_entries_by_tag() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileDiaryStore::new(temp_dir.path());
        let entry = sample_entry();
        store.append_entry(&entry).unwrap();

        let filter = DiaryFilter {
            partition: Some(DiaryPartition::Rational),
            domain: None,
            scope: None,
            since: None,
            until: None,
            tag: Some("design".into()),
            limit: Some(10),
        };
        let loaded = store.filter_entries(&filter).unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn test_list_days_descending() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileDiaryStore::new(temp_dir.path());

        let mut older = sample_entry();
        older.timestamp = DateTime::parse_from_rfc3339("2026-03-25T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        store.append_entry(&older).unwrap();

        let newer = sample_entry();
        store.append_entry(&newer).unwrap();

        let days = store.list_days(DiaryPartition::Rational).unwrap();
        assert_eq!(
            days,
            vec!["2026-03-26".to_string(), "2026-03-25".to_string()]
        );
    }
}
