use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::evolution::{EvidenceRef, EvidenceSource};
use crate::session::store::{ChatMessage, Session};

const DEFAULT_MAX_FILES_SCANNED: usize = 64;
const DEFAULT_MAX_RESULTS: usize = 12;
const DEFAULT_MAX_SNIPPET_CHARS: usize = 240;
const DEFAULT_MAX_TOTAL_BYTES: usize = 12 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSearchQuery {
    pub text: String,
    #[serde(default = "default_max_files_scanned")]
    pub max_files_scanned: usize,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    #[serde(default = "default_max_snippet_chars")]
    pub max_snippet_chars: usize,
    #[serde(default = "default_max_total_bytes")]
    pub max_total_bytes: usize,
}

impl SessionSearchQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            max_files_scanned: default_max_files_scanned(),
            max_results: default_max_results(),
            max_snippet_chars: default_max_snippet_chars(),
            max_total_bytes: default_max_total_bytes(),
        }
    }

    pub fn sanitized(self) -> Self {
        Self {
            text: self.text.trim().to_string(),
            max_files_scanned: self.max_files_scanned.max(1),
            max_results: self.max_results.max(1),
            max_snippet_chars: self.max_snippet_chars.clamp(32, 1_200),
            max_total_bytes: self.max_total_bytes.clamp(256, 256 * 1024),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSearchHit {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub snippet: String,
    pub source_uri: String,
    pub hash: String,
    pub source: EvidenceSource,
    pub snippet_truncated: bool,
    pub message_index: usize,
}

impl SessionSearchHit {
    pub fn to_evidence_ref(&self) -> EvidenceRef {
        EvidenceRef {
            id: format!("session-evidence-{}", self.hash),
            source: EvidenceSource::Session,
            uri: self.source_uri.clone(),
            excerpt: Some(self.snippet.clone()),
            hash: Some(self.hash.clone()),
            created_at: self.timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSearchDiagnostic {
    pub session_id: Option<String>,
    pub source_uri: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSearchResponse {
    pub hits: Vec<SessionSearchHit>,
    pub diagnostics: Vec<SessionSearchDiagnostic>,
    pub scanned_files: usize,
    pub skipped_files: usize,
    pub total_response_bytes: usize,
    pub file_limit_reached: bool,
    pub result_limit_reached: bool,
    pub byte_limit_reached: bool,
}

pub(crate) fn search_sessions_in_dir(
    sessions_dir: &Path,
    query: SessionSearchQuery,
) -> crate::Result<SessionSearchResponse> {
    let query = query.sanitized();
    if query.text.is_empty() || !sessions_dir.exists() {
        return Ok(SessionSearchResponse {
            hits: Vec::new(),
            diagnostics: Vec::new(),
            scanned_files: 0,
            skipped_files: 0,
            total_response_bytes: 0,
            file_limit_reached: false,
            result_limit_reached: false,
            byte_limit_reached: false,
        });
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(sessions_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
    files.sort();

    let file_limit_reached = files.len() > query.max_files_scanned;
    let mut hits = Vec::new();
    let mut diagnostics = Vec::new();
    let mut total_bytes = 0usize;
    let mut scanned_files = 0usize;
    let mut skipped_files = 0usize;
    let needle = query.text.to_lowercase();
    let mut result_limit_reached = false;
    let mut byte_limit_reached = false;

    for path in files.into_iter().take(query.max_files_scanned) {
        scanned_files += 1;
        match load_session_file(&path) {
            Ok((session_id, session)) => {
                for (message_index, message) in session.messages.iter().enumerate() {
                    if let Some(hit) = build_hit(
                        &needle,
                        &session_id,
                        message_index,
                        message,
                        query.max_snippet_chars,
                    ) {
                        let hit_bytes = estimated_hit_bytes(&hit);
                        if total_bytes + hit_bytes > query.max_total_bytes {
                            byte_limit_reached = true;
                            break;
                        }
                        total_bytes += hit_bytes;
                        hits.push(hit);
                        if hits.len() >= query.max_results {
                            result_limit_reached = true;
                            break;
                        }
                    }
                }
            }
            Err(reason) => {
                skipped_files += 1;
                diagnostics.push(SessionSearchDiagnostic {
                    session_id: path_to_session_id(&path),
                    source_uri: file_uri(&path),
                    reason,
                });
            }
        }

        if result_limit_reached || byte_limit_reached {
            break;
        }
    }

    Ok(SessionSearchResponse {
        hits,
        diagnostics,
        scanned_files,
        skipped_files,
        total_response_bytes: total_bytes,
        file_limit_reached,
        result_limit_reached,
        byte_limit_reached,
    })
}

fn load_session_file(path: &Path) -> Result<(String, Session), String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read session file {}: {error}", path.display()))?;
    let session_id = path_to_session_id(path)
        .ok_or_else(|| format!("invalid session file name: {}", path.display()))?;
    parse_session_jsonl(&session_id, &raw)
        .map(|session| (session_id, session))
        .map_err(|error| format!("failed to parse session file {}: {error}", path.display()))
}

fn parse_session_jsonl(session_id: &str, raw: &str) -> Result<Session, String> {
    let mut messages = Vec::new();
    let mut metadata = serde_json::Value::Object(serde_json::Map::new());
    let mut created_at = None;
    let mut last_consolidated = 0usize;
    let mut last_compacted = 0usize;
    let mut compaction_history = Vec::new();

    for (line_index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid JSON on line {}: {error}", line_index + 1))?;

        if value.get("_type").and_then(|value| value.as_str()) == Some("metadata") {
            metadata = value.get("metadata").cloned().unwrap_or(metadata);
            created_at = value
                .get("created_at")
                .and_then(|value| value.as_str())
                .and_then(|value| value.parse().ok());
            last_consolidated = value
                .get("last_consolidated")
                .and_then(|value| value.as_u64())
                .unwrap_or_default() as usize;
            last_compacted = value
                .get("last_compacted")
                .and_then(|value| value.as_u64())
                .unwrap_or_default() as usize;
            if let Some(history) = value.get("compaction_history") {
                compaction_history = serde_json::from_value(history.clone()).map_err(|error| {
                    format!(
                        "invalid compaction history on line {}: {error}",
                        line_index + 1
                    )
                })?;
            }
            continue;
        }

        let message: ChatMessage = serde_json::from_value(value)
            .map_err(|error| format!("invalid message on line {}: {error}", line_index + 1))?;
        messages.push(message);
    }

    Ok(Session {
        key: session_id.to_string(),
        messages,
        created_at: created_at.unwrap_or_else(Utc::now),
        updated_at: Utc::now(),
        metadata,
        last_consolidated,
        last_compacted,
        compaction_history,
    })
}

fn build_hit(
    needle: &str,
    session_id: &str,
    message_index: usize,
    message: &ChatMessage,
    max_snippet_chars: usize,
) -> Option<SessionSearchHit> {
    let content = message.content.trim();
    if content.is_empty() {
        return None;
    }
    let haystack = content.to_lowercase();
    let found = haystack.find(needle)?;
    let (snippet, snippet_truncated) =
        build_snippet(content, found, needle.chars().count(), max_snippet_chars);
    let source_uri = format!(
        "session://{}?message_index={message_index}",
        percent_encode(session_id)
    );
    let hash = stable_hash(&(session_id, message_index, &snippet));
    Some(SessionSearchHit {
        session_id: session_id.to_string(),
        timestamp: message.timestamp,
        snippet,
        source_uri,
        hash,
        source: EvidenceSource::Session,
        snippet_truncated,
        message_index,
    })
}

fn build_snippet(
    content: &str,
    byte_match_index: usize,
    needle_chars: usize,
    max_snippet_chars: usize,
) -> (String, bool) {
    let chars: Vec<char> = content.chars().collect();
    let prefix_chars = content[..byte_match_index].chars().count();
    let match_end = prefix_chars.saturating_add(needle_chars);
    let context = max_snippet_chars / 2;
    let start = prefix_chars.saturating_sub(context / 2);
    let mut end = (match_end + context).min(chars.len());
    if end.saturating_sub(start) > max_snippet_chars {
        end = start + max_snippet_chars;
    }
    let mut snippet: String = chars[start..end].iter().collect();
    let truncated = start > 0 || end < chars.len();
    if start > 0 {
        snippet = format!("...{snippet}");
    }
    if end < chars.len() {
        snippet.push_str("...");
    }
    (snippet, truncated)
}

fn estimated_hit_bytes(hit: &SessionSearchHit) -> usize {
    hit.session_id.len() + hit.snippet.len() + hit.source_uri.len() + hit.hash.len() + 64
}

fn stable_hash<T: Hash>(value: &T) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

fn path_to_session_id(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    Some(stem.replace('_', ":"))
}

fn file_uri(path: &Path) -> String {
    let path = path.to_string_lossy();
    format!("file://{path}")
}

const fn default_max_files_scanned() -> usize {
    DEFAULT_MAX_FILES_SCANNED
}

const fn default_max_results() -> usize {
    DEFAULT_MAX_RESULTS
}

const fn default_max_snippet_chars() -> usize {
    DEFAULT_MAX_SNIPPET_CHARS
}

const fn default_max_total_bytes() -> usize {
    DEFAULT_MAX_TOTAL_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_session_file(dir: &Path, name: &str, body: &str) {
        let path = dir.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    #[test]
    fn search_returns_structured_hits_with_session_evidence_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let sessions = temp.path().join("sessions");
        write_session_file(
            &sessions,
            "telegram_42.jsonl",
            r#"{"_type":"metadata","created_at":"2026-06-16T00:00:00Z","updated_at":"2026-06-16T00:00:00Z","metadata":{}}
{"role":"user","content":"Please remember the launch checklist for Friday review.","timestamp":"2026-06-16T01:02:03Z"}"#,
        );

        let result = search_sessions_in_dir(&sessions, SessionSearchQuery::new("launch")).unwrap();

        assert_eq!(result.hits.len(), 1);
        let hit = &result.hits[0];
        assert_eq!(hit.session_id, "telegram:42");
        assert_eq!(hit.source, EvidenceSource::Session);
        assert!(hit.snippet.contains("launch checklist"));
        assert_eq!(hit.source_uri, "session://telegram%3A42?message_index=0");
        assert!(!hit.hash.is_empty());
    }

    #[test]
    fn corrupted_files_are_skipped_with_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let sessions = temp.path().join("sessions");
        write_session_file(
            &sessions,
            "broken.jsonl",
            r#"{"_type":"metadata","created_at":"2026-06-16T00:00:00Z","updated_at":"2026-06-16T00:00:00Z","metadata":{}}
not-json"#,
        );

        let result =
            search_sessions_in_dir(&sessions, SessionSearchQuery::new("anything")).unwrap();

        assert!(result.hits.is_empty());
        assert_eq!(result.skipped_files, 1);
        assert_eq!(result.diagnostics.len(), 1);
        assert_eq!(result.diagnostics[0].session_id.as_deref(), Some("broken"));
        assert!(result.diagnostics[0].reason.contains("invalid JSON"));
    }

    #[test]
    fn search_bounds_results_and_reports_truncation_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let sessions = temp.path().join("sessions");
        write_session_file(
            &sessions,
            "alpha.jsonl",
            r#"{"_type":"metadata","created_at":"2026-06-16T00:00:00Z","updated_at":"2026-06-16T00:00:00Z","metadata":{}}
{"role":"user","content":"alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha","timestamp":"2026-06-16T01:02:03Z"}
{"role":"assistant","content":"alpha follow-up","timestamp":"2026-06-16T01:03:03Z"}"#,
        );

        let query = SessionSearchQuery {
            text: "alpha".to_string(),
            max_files_scanned: 1,
            max_results: 1,
            max_snippet_chars: 32,
            max_total_bytes: 256,
        };
        let result = search_sessions_in_dir(&sessions, query).unwrap();

        assert_eq!(result.hits.len(), 1);
        assert!(result.hits[0].snippet_truncated);
        assert!(result.result_limit_reached);
        assert!(result.total_response_bytes <= 256);
    }

    #[test]
    fn session_hits_convert_to_session_evidence_refs() {
        let hit = SessionSearchHit {
            session_id: "telegram:9".to_string(),
            timestamp: "2026-06-16T01:02:03Z".parse().unwrap(),
            snippet: "bounded snippet".to_string(),
            source_uri: "session://telegram%3A9?message_index=3".to_string(),
            hash: "cafebabe".to_string(),
            source: EvidenceSource::Session,
            snippet_truncated: false,
            message_index: 3,
        };

        let evidence = hit.to_evidence_ref();
        assert_eq!(evidence.source, EvidenceSource::Session);
        assert_eq!(evidence.uri, hit.source_uri);
        assert_eq!(evidence.hash.as_deref(), Some("cafebabe"));
        assert_eq!(evidence.excerpt.as_deref(), Some("bounded snippet"));
    }
}
