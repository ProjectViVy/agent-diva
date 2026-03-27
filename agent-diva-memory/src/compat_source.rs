//! Compatibility sources backed by legacy Markdown memory files.

use crate::compat::long_term_memory_file_path;
use crate::types::{MemoryDomain, MemoryRecord, MemoryScope, MemorySourceRef};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const DEFAULT_COMPAT_TITLE: &str = "Long-term memory compatibility layer";

#[derive(Debug, Clone)]
pub(crate) struct MemoryMdChunkSource {
    workspace: PathBuf,
}

impl MemoryMdChunkSource {
    pub(crate) fn new<P: AsRef<Path>>(workspace: P) -> Self {
        Self {
            workspace: workspace.as_ref().to_path_buf(),
        }
    }

    pub(crate) fn load_records(&self) -> agent_diva_core::Result<Vec<MemoryRecord>> {
        let path = long_term_memory_file_path(&self.workspace);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&path)?;
        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let timestamp = std::fs::metadata(&path)
            .and_then(|metadata| metadata.modified())
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| DateTime::<Utc>::from(SystemTime::UNIX_EPOCH));

        Ok(split_memory_markdown(trimmed, timestamp))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemoryChunk {
    title: String,
    section: Option<String>,
    content: String,
}

fn split_memory_markdown(content: &str, timestamp: DateTime<Utc>) -> Vec<MemoryRecord> {
    let chunks = extract_heading_chunks(content)
        .filter(|chunks| !chunks.is_empty())
        .unwrap_or_else(|| extract_paragraph_chunks(content));

    chunks
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| memory_record_from_chunk(index, chunk, timestamp))
        .collect()
}

fn extract_heading_chunks(content: &str) -> Option<Vec<MemoryChunk>> {
    let mut saw_heading = false;
    let mut chunks = Vec::new();
    let mut current_title: Option<String> = None;
    let mut current_body = Vec::new();

    for line in content.lines() {
        if let Some(title) = parse_heading(line) {
            saw_heading = true;
            flush_chunk(&mut chunks, current_title.take(), &current_body);
            current_title = Some(title);
            current_body.clear();
        } else {
            current_body.push(line);
        }
    }
    flush_chunk(&mut chunks, current_title, &current_body);

    saw_heading.then_some(chunks)
}

fn extract_paragraph_chunks(content: &str) -> Vec<MemoryChunk> {
    content
        .split("\n\n")
        .map(str::trim)
        .filter(|block| !is_noise_block(block))
        .enumerate()
        .map(|(index, block)| MemoryChunk {
            title: format!("Memory note {}", index + 1),
            section: None,
            content: block.to_string(),
        })
        .collect()
}

fn flush_chunk(chunks: &mut Vec<MemoryChunk>, title: Option<String>, body_lines: &[&str]) {
    let body = body_lines.join("\n").trim().to_string();
    if is_noise_block(&body) {
        return;
    }

    let title = title.unwrap_or_else(|| DEFAULT_COMPAT_TITLE.to_string());
    chunks.push(MemoryChunk {
        section: Some(title.clone()),
        title,
        content: body,
    });
}

fn parse_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('#') {
        return None;
    }
    let title = trimmed.trim_start_matches('#').trim();
    (!title.is_empty()).then(|| title.to_string())
}

fn is_noise_block(block: &str) -> bool {
    let trimmed = block.trim();
    if trimmed.is_empty() {
        return true;
    }

    !trimmed.chars().any(|ch| ch.is_alphanumeric() || is_cjk(ch))
}

fn summarize_chunk(chunk: &MemoryChunk) -> String {
    let mut summary = chunk
        .content
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or(chunk.title.as_str())
        .chars()
        .take(180)
        .collect::<String>();
    if summary.is_empty() {
        summary = chunk.title.clone();
    }
    summary
}

fn memory_record_from_chunk(
    index: usize,
    chunk: MemoryChunk,
    timestamp: DateTime<Utc>,
) -> MemoryRecord {
    let summary = summarize_chunk(&chunk);
    MemoryRecord {
        id: format!("compat-memory-md-{}", index + 1),
        timestamp,
        domain: MemoryDomain::Fact,
        scope: MemoryScope::Workspace,
        title: chunk.title,
        summary,
        content: chunk.content,
        tags: vec!["compat".into(), "memory-md".into(), "long-term".into()],
        source_refs: vec![MemorySourceRef {
            path: Some("memory/MEMORY.md".into()),
            section: chunk.section,
            note: Some("Compatibility layer chunk".into()),
        }],
        confidence: 0.7,
    }
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0xF900..=0xFAFF
            | 0x2F800..=0x2FA1F
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_records_chunks_memory_md_by_headings() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("memory")).unwrap();
        std::fs::write(
            temp.path().join("memory").join("MEMORY.md"),
            "# Long-term Memory\n\n## Preferences\nUse Chinese.\n\n## Decisions\nKeep core minimal.\n",
        )
        .unwrap();

        let source = MemoryMdChunkSource::new(temp.path());
        let records = source.load_records().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].title, "Preferences");
        assert_eq!(records[1].title, "Decisions");
        assert_eq!(
            records[0].source_refs[0].path.as_deref(),
            Some("memory/MEMORY.md")
        );
    }

    #[test]
    fn test_load_records_falls_back_to_paragraph_chunks() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join("memory")).unwrap();
        std::fs::write(
            temp.path().join("memory").join("MEMORY.md"),
            "Use Chinese in replies.\n\nKeep compatibility with MEMORY.md.\n",
        )
        .unwrap();

        let source = MemoryMdChunkSource::new(temp.path());
        let records = source.load_records().unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].title, "Memory note 1");
        assert!(records[1].summary.contains("Keep compatibility"));
    }

    #[test]
    fn test_noise_blocks_are_filtered() {
        let timestamp = DateTime::<Utc>::from(SystemTime::UNIX_EPOCH);
        let records = split_memory_markdown("# Header\n\n---\n\n## Real\nKeep this.\n", timestamp);
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].title, "Real");
    }
}
