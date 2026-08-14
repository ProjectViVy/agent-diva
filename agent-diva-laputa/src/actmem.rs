//! Machine-wide ACTMEM authority and capsule lifecycle.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::UNIX_EPOCH,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::atomic_write;

pub const ACTMEM_FILE_NAME: &str = "ACTMEM.MD";
pub const ACTMEM_RING_CAP_CHARS: usize = 1_600;
pub const ACTMEM_WORK_CAP_CHARS: usize = 1_600;
pub const ACTMEM_READ_CAP_CHARS: usize = 1_200;
pub const ACTMEM_CAPSULE_CAP_CHARS: usize = 800;
pub const PULSE_ITEM_CAP_CHARS: usize = 280;
pub const RECAP_ITEM_CAP_CHARS: usize = 200;
pub const WORK_SECTIONS: [&str; 5] = ["Goal", "Open", "Next", "Constraints", "Pointers"];

#[derive(Debug, thiserror::Error)]
pub enum ActmemError {
    #[error("ACTMEM I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("ACTMEM document is malformed: {0}")]
    Malformed(String),
    #[error("ACTMEM revision conflict: expected {expected}, actual {actual}")]
    RevisionConflict { expected: u64, actual: u64 },
    #[error("ACTMEM section exceeds its capacity: {section}")]
    CapacityExceeded { section: &'static str },
    #[error("unknown ACTMEM Work section: {0}")]
    UnknownWorkSection(String),
    #[error("ACTMEM item index is out of range")]
    ItemIndexOutOfRange,
    #[error("ACTMEM capsule is not part of the server projection")]
    InvalidCapsule,
}

impl ActmemError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::RevisionConflict { .. } => "actmem_revision_conflict",
            Self::CapacityExceeded { .. } => "actmem_cap_exceeded",
            Self::InvalidCapsule => "actmem_capsule_invalid",
            Self::Malformed(_) => "actmem_malformed",
            Self::Io { .. } => "actmem_io_error",
            Self::UnknownWorkSection(_) | Self::ItemIndexOutOfRange => "actmem_invalid_edit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActmemDocument {
    pub revision: u64,
    pub updated_at: DateTime<Utc>,
    pub pulse: String,
    pub recap: String,
    pub work: String,
    pub markdown: String,
}

impl ActmemDocument {
    pub fn empty() -> Self {
        let updated_at = DateTime::<Utc>::from(UNIX_EPOCH);
        let mut document = Self {
            revision: 0,
            updated_at,
            pulse: String::new(),
            recap: String::new(),
            work: empty_work(),
            markdown: String::new(),
        };
        document.markdown = render(&document);
        document
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ActmemPatch {
    pub pulse: Option<String>,
    pub recap: Option<String>,
    pub work: Option<String>,
    pub base_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapsuleSummary {
    pub name: String,
    pub session_key: String,
    pub created_at: DateTime<Utc>,
    pub chars: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapsuleDocument {
    pub name: String,
    pub session_key: String,
    pub created_at: DateTime<Utc>,
    pub markdown: String,
}

#[derive(Debug, Clone)]
pub struct ActmemStore {
    root: PathBuf,
    write_lock: Arc<tokio::sync::Mutex<()>>,
}

impl ActmemStore {
    pub fn new(config_dir: impl AsRef<Path>) -> Self {
        Self {
            root: config_dir.as_ref().join("actmem"),
            write_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn head_path(&self) -> PathBuf {
        self.root.join(ACTMEM_FILE_NAME)
    }

    pub fn capsules_dir(&self) -> PathBuf {
        self.root.join("capsules")
    }

    pub fn read(&self) -> Result<ActmemDocument, ActmemError> {
        read_document(&self.head_path())
    }

    pub async fn put(&self, patch: ActmemPatch) -> Result<ActmemDocument, ActmemError> {
        let _guard = self.write_lock.lock().await;
        let current = self.read()?;
        if current.revision != patch.base_revision {
            return Err(ActmemError::RevisionConflict {
                expected: patch.base_revision,
                actual: current.revision,
            });
        }
        let mut next = current.clone();
        if let Some(pulse) = patch.pulse {
            next.pulse = normalize_section(&pulse);
        }
        if let Some(recap) = patch.recap {
            next.recap = normalize_section(&recap);
        }
        if let Some(work) = patch.work {
            next.work = normalize_work(&work)?;
        }
        validate_caps(&next)?;
        self.commit_if_changed(current, next)
    }

    pub async fn append_pulse(
        &self,
        session_key: &str,
        content: &str,
    ) -> Result<ActmemDocument, ActmemError> {
        self.append_ring(session_key, content, RingKind::Pulse)
            .await
    }

    pub async fn append_recap(
        &self,
        session_key: &str,
        content: &str,
    ) -> Result<ActmemDocument, ActmemError> {
        self.append_ring(session_key, content, RingKind::Recap)
            .await
    }

    async fn append_ring(
        &self,
        session_key: &str,
        content: &str,
        kind: RingKind,
    ) -> Result<ActmemDocument, ActmemError> {
        let content = normalize_visible(content);
        if content.is_empty() {
            return self.read();
        }
        let _guard = self.write_lock.lock().await;
        let current = self.read()?;
        let limit = match kind {
            RingKind::Pulse => PULSE_ITEM_CAP_CHARS,
            RingKind::Recap => RECAP_ITEM_CAP_CHARS,
        };
        let item = format!(
            "- {} {} <!-- session-hex:{} -->",
            Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            truncate_chars(&content, limit),
            hex_session(session_key)
        );
        let mut next = current.clone();
        let section = match kind {
            RingKind::Pulse => &mut next.pulse,
            RingKind::Recap => &mut next.recap,
        };
        let mut lines = section
            .lines()
            .map(str::to_string)
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        lines.push(item);
        while lines.join("\n").chars().count() > ACTMEM_RING_CAP_CHARS && lines.len() > 1 {
            lines.remove(0);
        }
        *section = lines.join("\n");
        self.commit_if_changed(current, next)
    }

    pub async fn edit_work(
        &self,
        section: &str,
        replacement: &str,
        base_revision: u64,
    ) -> Result<ActmemDocument, ActmemError> {
        let _guard = self.write_lock.lock().await;
        let current = self.read()?;
        ensure_revision(&current, base_revision)?;
        let mut sections = split_work(&current.work)?;
        let canonical = canonical_work_section(section)?;
        sections.insert(canonical.to_string(), normalize_section(replacement));
        let mut next = current.clone();
        next.work = render_work_sections(&sections);
        validate_caps(&next)?;
        self.commit_if_changed(current, next)
    }

    pub async fn complete_open_item(
        &self,
        item_index: usize,
        base_revision: u64,
    ) -> Result<ActmemDocument, ActmemError> {
        self.drop_item("Open", item_index, base_revision).await
    }

    pub async fn drop_item(
        &self,
        section: &str,
        item_index: usize,
        base_revision: u64,
    ) -> Result<ActmemDocument, ActmemError> {
        let _guard = self.write_lock.lock().await;
        let current = self.read()?;
        ensure_revision(&current, base_revision)?;
        let mut sections = split_work(&current.work)?;
        let canonical = canonical_work_section(section)?;
        let body = sections.get(canonical).cloned().unwrap_or_default();
        let mut items = body.lines().map(str::to_string).collect::<Vec<_>>();
        if item_index >= items.len() {
            return Err(ActmemError::ItemIndexOutOfRange);
        }
        items.remove(item_index);
        sections.insert(canonical.to_string(), items.join("\n"));
        let mut next = current.clone();
        next.work = render_work_sections(&sections);
        self.commit_if_changed(current, next)
    }

    pub async fn fold_session(
        &self,
        session_key: &str,
    ) -> Result<Vec<CapsuleSummary>, ActmemError> {
        let _guard = self.write_lock.lock().await;
        let current = self.read()?;
        let marker = format!("<!-- session-hex:{} -->", hex_session(session_key));
        let (pulse_kept, pulse_folded) = partition_lines(&current.pulse, &marker);
        let (recap_kept, recap_folded) = partition_lines(&current.recap, &marker);
        let folded = pulse_folded
            .into_iter()
            .map(|line| format!("Pulse: {line}"))
            .chain(
                recap_folded
                    .into_iter()
                    .map(|line| format!("Recap: {line}")),
            )
            .collect::<Vec<_>>();
        if folded.is_empty() {
            return Ok(Vec::new());
        }

        let chunks = chunk_lines(&folded, 560);
        let mut summaries = Vec::with_capacity(chunks.len());
        for (index, chunk) in chunks.iter().enumerate() {
            summaries.push(self.write_capsule(session_key, index, chunk)?);
        }
        let mut next = current.clone();
        next.pulse = pulse_kept.join("\n");
        next.recap = recap_kept.join("\n");
        self.commit_if_changed(current, next)?;
        Ok(summaries)
    }

    pub fn list_capsules(&self) -> Result<Vec<CapsuleSummary>, ActmemError> {
        let dir = self.capsules_dir();
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(io_error(&dir, source)),
        };
        let mut capsules = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| io_error(&dir, source))?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("md") {
                continue;
            }
            let raw = fs::read_to_string(&path).map_err(|source| io_error(&path, source))?;
            let document = parse_capsule(&path, raw)?;
            capsules.push(CapsuleSummary {
                name: document.name,
                session_key: document.session_key,
                created_at: document.created_at,
                chars: document.markdown.chars().count(),
            });
        }
        capsules.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(capsules)
    }

    pub fn read_capsule(&self, name: &str) -> Result<CapsuleDocument, ActmemError> {
        let summary = self
            .list_capsules()?
            .into_iter()
            .find(|capsule| capsule.name == name)
            .ok_or(ActmemError::InvalidCapsule)?;
        let path = self.capsules_dir().join(&summary.name);
        let raw = fs::read_to_string(&path).map_err(|source| io_error(&path, source))?;
        parse_capsule(&path, raw)
    }

    pub async fn delete_capsule(&self, name: &str) -> Result<(), ActmemError> {
        let _guard = self.write_lock.lock().await;
        let summary = self
            .list_capsules()?
            .into_iter()
            .find(|capsule| capsule.name == name)
            .ok_or(ActmemError::InvalidCapsule)?;
        let path = self.capsules_dir().join(summary.name);
        fs::remove_file(&path).map_err(|source| io_error(path, source))
    }

    fn write_capsule(
        &self,
        session_key: &str,
        chunk_index: usize,
        body: &str,
    ) -> Result<CapsuleSummary, ActmemError> {
        let created_at = Utc::now();
        let millis = created_at.timestamp_millis();
        let safe = safe_session_key(session_key);
        let mut suffix = chunk_index;
        loop {
            let name = format!("{safe}_{millis}_{suffix}.md");
            let path = self.capsules_dir().join(&name);
            if path.exists() {
                suffix += 1;
                continue;
            }
            let markdown = format!(
                "---\nsession_key: {}\ncreated_at: {}\n---\n\n# ACTMEM Capsule\n\n{}\n",
                quote_front_matter(session_key),
                created_at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                body.trim()
            );
            if markdown.chars().count() > ACTMEM_CAPSULE_CAP_CHARS {
                return Err(ActmemError::CapacityExceeded { section: "capsule" });
            }
            atomic_write(&path, markdown.as_bytes())
                .map_err(|error| ActmemError::Malformed(error.to_string()))?;
            return Ok(CapsuleSummary {
                name,
                session_key: session_key.to_string(),
                created_at,
                chars: markdown.chars().count(),
            });
        }
    }

    fn commit_if_changed(
        &self,
        current: ActmemDocument,
        mut next: ActmemDocument,
    ) -> Result<ActmemDocument, ActmemError> {
        if current.pulse == next.pulse && current.recap == next.recap && current.work == next.work {
            return Ok(current);
        }
        next.revision = current.revision + 1;
        next.updated_at = Utc::now();
        next.markdown = render(&next);
        atomic_write(self.head_path(), next.markdown.as_bytes())
            .map_err(|error| ActmemError::Malformed(error.to_string()))?;
        Ok(next)
    }
}

#[derive(Clone, Copy)]
enum RingKind {
    Pulse,
    Recap,
}

fn read_document(path: &Path) -> Result<ActmemDocument, ActmemError> {
    match fs::read_to_string(path) {
        Ok(raw) => parse(&raw),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(ActmemDocument::empty()),
        Err(source) => Err(io_error(path, source)),
    }
}

fn parse(raw: &str) -> Result<ActmemDocument, ActmemError> {
    let normalized = raw.replace("\r\n", "\n");
    let mut lines = normalized.lines();
    if lines.next() != Some("---") {
        return Err(ActmemError::Malformed("missing front matter".into()));
    }
    let revision = parse_front_value(&mut lines, "revision")?
        .parse::<u64>()
        .map_err(|_| ActmemError::Malformed("invalid revision".into()))?;
    let updated_at = DateTime::parse_from_rfc3339(&parse_front_value(&mut lines, "updated_at")?)
        .map_err(|_| ActmemError::Malformed("invalid updated_at".into()))?
        .with_timezone(&Utc);
    if lines.next() != Some("---") || lines.next() != Some("") || lines.next() != Some("# ACTMEM") {
        return Err(ActmemError::Malformed("invalid ACTMEM heading".into()));
    }
    let body = lines.collect::<Vec<_>>().join("\n");
    let (pulse, rest) = take_section(&body, "## Pulse", "## Recap")?;
    let (recap, work) = take_section(rest, "## Recap", "## Work")?;
    let work = work
        .strip_prefix("## Work")
        .ok_or_else(|| ActmemError::Malformed("missing Work heading".into()))?;
    let work = normalize_work(work.trim_start_matches('\n'))?;
    let mut document = ActmemDocument {
        revision,
        updated_at,
        pulse: normalize_section(pulse),
        recap: normalize_section(recap),
        work,
        markdown: normalized.trim_end().to_string() + "\n",
    };
    validate_caps(&document)?;
    document.markdown = render(&document);
    Ok(document)
}

fn parse_front_value<'a>(
    lines: &mut impl Iterator<Item = &'a str>,
    key: &str,
) -> Result<String, ActmemError> {
    let line = lines
        .next()
        .ok_or_else(|| ActmemError::Malformed(format!("missing {key}")))?;
    let prefix = format!("{key}: ");
    line.strip_prefix(&prefix)
        .map(str::to_string)
        .ok_or_else(|| ActmemError::Malformed(format!("invalid {key}")))
}

fn take_section<'a>(
    body: &'a str,
    heading: &str,
    next: &str,
) -> Result<(&'a str, &'a str), ActmemError> {
    let body = body
        .strip_prefix(&format!("\n{heading}\n"))
        .or_else(|| body.strip_prefix(&format!("{heading}\n")))
        .ok_or_else(|| ActmemError::Malformed(format!("missing {heading}")))?;
    let marker = format!("\n{next}\n");
    let index = body
        .find(&marker)
        .ok_or_else(|| ActmemError::Malformed(format!("missing {next}")))?;
    Ok((&body[..index], &body[index + 1..]))
}

fn render(document: &ActmemDocument) -> String {
    format!(
        "---\nrevision: {}\nupdated_at: {}\n---\n\n# ACTMEM\n\n## Pulse\n{}\n\n## Recap\n{}\n\n## Work\n{}\n",
        document.revision,
        document
            .updated_at
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        document.pulse.trim(),
        document.recap.trim(),
        document.work.trim()
    )
}

fn validate_caps(document: &ActmemDocument) -> Result<(), ActmemError> {
    if document.pulse.chars().count() > ACTMEM_RING_CAP_CHARS {
        return Err(ActmemError::CapacityExceeded { section: "pulse" });
    }
    if document.recap.chars().count() > ACTMEM_RING_CAP_CHARS {
        return Err(ActmemError::CapacityExceeded { section: "recap" });
    }
    if document.work.chars().count() > ACTMEM_WORK_CAP_CHARS {
        return Err(ActmemError::CapacityExceeded { section: "work" });
    }
    Ok(())
}

fn ensure_revision(document: &ActmemDocument, expected: u64) -> Result<(), ActmemError> {
    if document.revision == expected {
        Ok(())
    } else {
        Err(ActmemError::RevisionConflict {
            expected,
            actual: document.revision,
        })
    }
}

fn empty_work() -> String {
    WORK_SECTIONS
        .iter()
        .map(|name| format!("### {name}"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn normalize_work(raw: &str) -> Result<String, ActmemError> {
    let sections = split_work(raw)?;
    Ok(render_work_sections(&sections))
}

fn split_work(raw: &str) -> Result<std::collections::BTreeMap<String, String>, ActmemError> {
    let normalized = raw.replace("\r\n", "\n");
    let mut sections = std::collections::BTreeMap::new();
    let mut current: Option<String> = None;
    let mut body = Vec::new();
    for line in normalized.lines() {
        if let Some(name) = line.strip_prefix("### ") {
            if let Some(previous) = current.take() {
                sections.insert(previous, normalize_section(&body.join("\n")));
                body.clear();
            }
            let canonical = canonical_work_section(name)?;
            if sections.contains_key(canonical) || current.as_deref() == Some(canonical) {
                return Err(ActmemError::Malformed(format!(
                    "duplicate Work section {canonical}"
                )));
            }
            current = Some(canonical.to_string());
        } else if current.is_some() {
            body.push(line);
        } else if !line.trim().is_empty() {
            return Err(ActmemError::Malformed(
                "Work content must be under a registered subsection".into(),
            ));
        }
    }
    if let Some(previous) = current {
        sections.insert(previous, normalize_section(&body.join("\n")));
    }
    for name in WORK_SECTIONS {
        sections.entry(name.to_string()).or_default();
    }
    Ok(sections)
}

fn render_work_sections(sections: &std::collections::BTreeMap<String, String>) -> String {
    WORK_SECTIONS
        .iter()
        .map(|name| {
            let body = sections.get(*name).map(String::as_str).unwrap_or_default();
            if body.is_empty() {
                format!("### {name}")
            } else {
                format!("### {name}\n{body}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn canonical_work_section(section: &str) -> Result<&'static str, ActmemError> {
    WORK_SECTIONS
        .iter()
        .copied()
        .find(|candidate| candidate.eq_ignore_ascii_case(section.trim()))
        .ok_or_else(|| ActmemError::UnknownWorkSection(section.to_string()))
}

fn normalize_section(value: &str) -> String {
    value.replace("\r\n", "\n").trim().to_string()
}

fn normalize_visible(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("```"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn recap_from_final_response(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n");
    let mut in_code = false;
    let mut paragraph = Vec::new();
    for line in normalized.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        let visible = trimmed
            .trim_start_matches('#')
            .trim_start_matches(['-', '*', '>'])
            .trim();
        if !visible.is_empty() {
            paragraph.push(visible);
        }
    }
    truncate_chars(&paragraph.join(" "), RECAP_ITEM_CAP_CHARS)
}

fn truncate_chars(value: &str, max: usize) -> String {
    let mut result = value.chars().take(max).collect::<String>();
    if value.chars().count() > max && max > 0 {
        result.pop();
        result.push('…');
    }
    result
}

fn partition_lines(value: &str, marker: &str) -> (Vec<String>, Vec<String>) {
    value
        .lines()
        .map(str::to_string)
        .partition(|line| !line.contains(marker))
}

fn chunk_lines(lines: &[String], target_chars: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in lines {
        let line = truncate_chars(line, target_chars);
        let extra = line.chars().count() + usize::from(!current.is_empty());
        if !current.is_empty() && current.chars().count() + extra > target_chars {
            chunks.push(current);
            current = String::new();
        }
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(&line);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn safe_session_key(session_key: &str) -> String {
    let safe = session_key
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if safe.is_empty() {
        "session".to_string()
    } else {
        safe
    }
}

fn hex_session(session_key: &str) -> String {
    session_key
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn quote_front_matter(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"session\"".to_string())
}

fn parse_capsule(path: &Path, markdown: String) -> Result<CapsuleDocument, ActmemError> {
    if markdown.chars().count() > ACTMEM_CAPSULE_CAP_CHARS {
        return Err(ActmemError::CapacityExceeded { section: "capsule" });
    }
    let mut lines = markdown.lines();
    if lines.next() != Some("---") {
        return Err(ActmemError::Malformed(
            "capsule front matter missing".into(),
        ));
    }
    let session_line = lines
        .next()
        .and_then(|line| line.strip_prefix("session_key: "))
        .ok_or_else(|| ActmemError::Malformed("capsule session_key missing".into()))?;
    let session_key = serde_json::from_str::<String>(session_line)
        .map_err(|_| ActmemError::Malformed("capsule session_key invalid".into()))?;
    let created_line = lines
        .next()
        .and_then(|line| line.strip_prefix("created_at: "))
        .ok_or_else(|| ActmemError::Malformed("capsule created_at missing".into()))?;
    let created_at = DateTime::parse_from_rfc3339(created_line)
        .map_err(|_| ActmemError::Malformed("capsule created_at invalid".into()))?
        .with_timezone(&Utc);
    if lines.next() != Some("---") {
        return Err(ActmemError::Malformed(
            "capsule front matter not closed".into(),
        ));
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ActmemError::InvalidCapsule)?
        .to_string();
    Ok(CapsuleDocument {
        name,
        session_key,
        created_at,
        markdown,
    })
}

fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> ActmemError {
    ActmemError::Io {
        path: path.into(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_head_is_empty_without_creating_directories() {
        let temp = tempfile::tempdir().unwrap();
        let store = ActmemStore::new(temp.path());
        let document = store.read().unwrap();
        assert_eq!(document.revision, 0);
        assert!(!store.root().exists());
    }

    #[tokio::test]
    async fn append_is_bounded_and_noop_does_not_advance_revision() {
        let temp = tempfile::tempdir().unwrap();
        let store = ActmemStore::new(temp.path());
        let first = store
            .append_pulse("gui:one", &"x".repeat(400))
            .await
            .unwrap();
        assert_eq!(first.revision, 1);
        assert!(first.pulse.chars().count() <= ACTMEM_RING_CAP_CHARS);
        let noop = store
            .put(ActmemPatch {
                pulse: Some(first.pulse.clone()),
                recap: None,
                work: None,
                base_revision: first.revision,
            })
            .await
            .unwrap();
        assert_eq!(noop.revision, first.revision);
    }

    #[tokio::test]
    async fn user_put_enforces_cas_and_work_capacity() {
        let temp = tempfile::tempdir().unwrap();
        let store = ActmemStore::new(temp.path());
        let first = store.append_pulse("gui:one", "hello").await.unwrap();
        let conflict = store
            .put(ActmemPatch {
                base_revision: 0,
                ..ActmemPatch::default()
            })
            .await
            .unwrap_err();
        assert_eq!(conflict.code(), "actmem_revision_conflict");
        let too_large = format!("### Goal\n{}", "x".repeat(2_000));
        let error = store
            .put(ActmemPatch {
                work: Some(too_large),
                base_revision: first.revision,
                ..ActmemPatch::default()
            })
            .await
            .unwrap_err();
        assert_eq!(error.code(), "actmem_cap_exceeded");
    }

    #[tokio::test]
    async fn fold_writes_bounded_capsules_then_removes_session_lines() {
        let temp = tempfile::tempdir().unwrap();
        let store = ActmemStore::new(temp.path());
        store.append_pulse("gui:one", "first").await.unwrap();
        store.append_recap("gui:one", "done").await.unwrap();
        store.append_pulse("gui:two", "keep").await.unwrap();
        let capsules = store.fold_session("gui:one").await.unwrap();
        assert!(!capsules.is_empty());
        assert!(capsules
            .iter()
            .all(|capsule| capsule.chars <= ACTMEM_CAPSULE_CAP_CHARS));
        let head = store.read().unwrap();
        assert!(!head.pulse.contains("first"));
        assert!(!head.recap.contains("done"));
        assert!(head.pulse.contains("keep"));
    }

    #[test]
    fn recap_uses_first_non_code_visible_paragraph() {
        let recap = recap_from_final_response("```rs\nignored\n```\n\n完成了主路径。\n\n后文");
        assert_eq!(recap, "完成了主路径。");
    }
}
