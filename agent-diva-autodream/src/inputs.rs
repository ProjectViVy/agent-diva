use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
};

use agent_diva_core::{
    evolution::{
        AutoDreamInputOmission, AutoDreamInputSourceSummary, AutoDreamInputSummary, EvidenceRef,
        EvidenceSource, LaputaSectionName,
    },
    session::{Session, SessionManager},
};
use agent_diva_laputa::LaputaService;
use chrono::Utc;
use uuid::Uuid;

use crate::{AutoDreamError, AutoDreamStorage, Result};

const DEFAULT_SESSION_LIMIT: usize = 3;
const DEFAULT_SESSION_BYTES: usize = 4096;
const DEFAULT_LAPUTA_SECTION_LIMIT: usize = 3;
const DEFAULT_LAPUTA_SECTION_BYTES: usize = 2048;
const DEFAULT_CAPSULE_LIMIT: usize = 2;
const DEFAULT_CAPSULE_BYTES: usize = 2048;
const DEFAULT_TOTAL_BYTES: usize = 8192;
const SESSION_SOURCE: &str = "recent_sessions";
const LAPUTA_SOURCE: &str = "laputa";
const CAPSULE_SOURCE: &str = "source_capsules";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoDreamInputCollectorConfig {
    pub recent_session_limit: usize,
    pub recent_session_bytes: usize,
    pub laputa_section_limit: usize,
    pub laputa_section_bytes: usize,
    pub capsule_limit: usize,
    pub capsule_bytes: usize,
    pub total_bytes_budget: usize,
    pub laputa_sections: Vec<LaputaSectionName>,
}

impl Default for AutoDreamInputCollectorConfig {
    fn default() -> Self {
        Self {
            recent_session_limit: DEFAULT_SESSION_LIMIT,
            recent_session_bytes: DEFAULT_SESSION_BYTES,
            laputa_section_limit: DEFAULT_LAPUTA_SECTION_LIMIT,
            laputa_section_bytes: DEFAULT_LAPUTA_SECTION_BYTES,
            capsule_limit: DEFAULT_CAPSULE_LIMIT,
            capsule_bytes: DEFAULT_CAPSULE_BYTES,
            total_bytes_budget: DEFAULT_TOTAL_BYTES,
            laputa_sections: vec![
                LaputaSectionName::MemoryMd,
                LaputaSectionName::JournalReflective,
                LaputaSectionName::Identity,
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoDreamCollectedInput {
    pub source: String,
    pub uri: String,
    pub excerpt: String,
    pub bytes: usize,
    pub truncated: bool,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoDreamCollectedInputs {
    pub items: Vec<AutoDreamCollectedInput>,
    pub summary: AutoDreamInputSummary,
}

#[derive(Debug, Clone)]
pub struct AutoDreamInputCollector {
    storage: AutoDreamStorage,
    laputa: LaputaService,
    config: AutoDreamInputCollectorConfig,
}

impl AutoDreamInputCollector {
    pub fn new(storage: AutoDreamStorage, laputa: LaputaService) -> Self {
        Self {
            storage,
            laputa,
            config: AutoDreamInputCollectorConfig::default(),
        }
    }

    pub fn with_config(mut self, config: AutoDreamInputCollectorConfig) -> Self {
        self.config = config;
        self
    }

    pub fn collect(&self, run_id: &str) -> Result<AutoDreamCollectedInputs> {
        let mut items = Vec::new();
        let mut omissions = Vec::new();
        let mut source_summaries = Vec::new();
        let mut remaining_budget = self.config.total_bytes_budget;
        let mut any_truncated = false;

        let session_items = self.collect_recent_sessions(&mut omissions)?;
        let (included, summary, truncated) =
            apply_budget(SESSION_SOURCE, session_items, &mut remaining_budget);
        any_truncated |= truncated;
        items.extend(included);
        source_summaries.push(summary);

        let laputa_items = self.collect_laputa_sections(&mut omissions)?;
        let (included, summary, truncated) =
            apply_budget(LAPUTA_SOURCE, laputa_items, &mut remaining_budget);
        any_truncated |= truncated;
        items.extend(included);
        source_summaries.push(summary);

        let capsule_items = self.collect_capsules(&mut omissions)?;
        let (included, summary, truncated) =
            apply_budget(CAPSULE_SOURCE, capsule_items, &mut remaining_budget);
        any_truncated |= truncated;
        items.extend(included);
        source_summaries.push(summary);

        if items.is_empty() {
            return Err(AutoDreamError::InputCollection(
                "all mandatory inputs omitted".to_string(),
            ));
        }

        let total_bytes = items.iter().map(|item| item.bytes).sum();
        let summary = AutoDreamInputSummary {
            total_items: items.len(),
            included_sources: source_summaries,
            omissions,
            truncated: any_truncated,
            total_bytes,
        };

        let _ = run_id;
        Ok(AutoDreamCollectedInputs { items, summary })
    }

    fn collect_recent_sessions(
        &self,
        omissions: &mut Vec<AutoDreamInputOmission>,
    ) -> Result<Vec<AutoDreamCollectedInput>> {
        let mut manager = SessionManager::new(self.storage.paths().workspace_root());
        let infos = read_session_candidates(&self.storage)?;
        if infos.is_empty() {
            omissions.push(omission(SESSION_SOURCE, "no sessions found"));
            return Ok(Vec::new());
        }

        let selected = infos.into_iter().take(self.config.recent_session_limit);
        let mut items = Vec::new();
        for info in selected {
            match manager.get_or_load(&info.key) {
                Some(session) => {
                    if let Some(item) = session_to_input(session, self.config.recent_session_bytes)
                    {
                        items.push(item);
                    } else {
                        let detail = match fs::read_to_string(&info.path) {
                            Ok(raw) if !raw.trim().is_empty() => {
                                format!("session {} omitted due to corrupt file", info.key)
                            }
                            _ => format!("session {} had no readable messages", info.key),
                        };
                        omissions.push(omission(SESSION_SOURCE, detail));
                    }
                }
                None => {
                    let detail = if Path::new(&info.path).exists() {
                        format!("session {} omitted due to corrupt file", info.key)
                    } else {
                        format!("session {} omitted due to missing file", info.key)
                    };
                    omissions.push(omission(SESSION_SOURCE, detail));
                }
            }
        }
        Ok(items)
    }

    fn collect_laputa_sections(
        &self,
        omissions: &mut Vec<AutoDreamInputOmission>,
    ) -> Result<Vec<AutoDreamCollectedInput>> {
        let sections = self
            .config
            .laputa_sections
            .iter()
            .take(self.config.laputa_section_limit);
        let mut items = Vec::new();

        for section in sections {
            let section_data = self
                .laputa
                .read_section(section.clone())
                .map_err(|error| AutoDreamError::InputCollection(error.to_string()))?;
            if section_data.content.is_null() {
                omissions.push(omission(
                    LAPUTA_SOURCE,
                    format!("section {} absent", section.as_str()),
                ));
                continue;
            }

            let raw = if section_data.content.is_string() {
                section_data
                    .content
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            } else {
                serde_json::to_string_pretty(&section_data.content)?
            };

            let excerpt = truncate_text(&raw, self.config.laputa_section_bytes);
            items.push(AutoDreamCollectedInput {
                source: LAPUTA_SOURCE.to_string(),
                uri: format!("laputa://section/{}", section.as_str()),
                bytes: excerpt.len(),
                truncated: excerpt.len() < raw.len(),
                evidence: evidence_ref(
                    EvidenceSource::LaputaSection,
                    format!("laputa://section/{}", section.as_str()),
                    Some(excerpt.clone()),
                ),
                excerpt,
            });
        }

        Ok(items)
    }

    fn collect_capsules(
        &self,
        omissions: &mut Vec<AutoDreamInputOmission>,
    ) -> Result<Vec<AutoDreamCollectedInput>> {
        let capsules_dir = self.storage.paths().compact_capsules_dir();
        if !capsules_dir.exists() {
            omissions.push(omission(
                CAPSULE_SOURCE,
                "capsules directory absent".to_string(),
            ));
            return Ok(Vec::new());
        }

        let mut entries = read_dir_sorted(&capsules_dir)?;
        if entries.is_empty() {
            omissions.push(omission(CAPSULE_SOURCE, "no capsule files found"));
            return Ok(Vec::new());
        }

        entries.truncate(self.config.capsule_limit);
        let mut items = Vec::new();
        for path in entries {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    let excerpt = truncate_text(&content, self.config.capsule_bytes);
                    items.push(AutoDreamCollectedInput {
                        source: CAPSULE_SOURCE.to_string(),
                        uri: format!("capsule://{}", path.file_name().unwrap().to_string_lossy()),
                        bytes: excerpt.len(),
                        truncated: excerpt.len() < content.len(),
                        evidence: evidence_ref(
                            EvidenceSource::ContextCompaction,
                            format!("capsule://{}", path.file_name().unwrap().to_string_lossy()),
                            Some(excerpt.clone()),
                        ),
                        excerpt,
                    });
                }
                Err(error) => omissions.push(omission(
                    CAPSULE_SOURCE,
                    format!("failed to read {}: {error}", path.display()),
                )),
            }
        }

        Ok(items)
    }
}

fn read_session_candidates(
    storage: &AutoDreamStorage,
) -> Result<Vec<agent_diva_core::session::SessionInfo>> {
    let sessions_dir = storage.paths().sessions_dir();
    if !sessions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(&sessions_dir)
        .map_err(|source| AutoDreamError::io(&sessions_dir, source))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|source| AutoDreamError::io(&sessions_dir, source))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
        .collect::<Vec<_>>();

    entries.sort_by(|left, right| right.cmp(left));

    Ok(entries
        .into_iter()
        .map(|path| {
            let name = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .replace('_', ":");
            agent_diva_core::session::SessionInfo {
                key: name,
                created_at: None,
                updated_at: None,
                path: path.to_string_lossy().to_string(),
            }
        })
        .collect())
}

fn session_to_input(session: &Session, max_bytes: usize) -> Option<AutoDreamCollectedInput> {
    let mut lines = VecDeque::new();
    for message in session.get_history(16) {
        let content = message.content.trim();
        if content.is_empty() {
            continue;
        }
        lines.push_back(format!("{}: {}", message.role, content));
    }
    if lines.is_empty() {
        return None;
    }
    let joined = lines.into_iter().collect::<Vec<_>>().join("\n");
    let excerpt = truncate_text(&joined, max_bytes);
    Some(AutoDreamCollectedInput {
        source: SESSION_SOURCE.to_string(),
        uri: format!("session://{}", session.key),
        bytes: excerpt.len(),
        truncated: excerpt.len() < joined.len(),
        evidence: evidence_ref(
            EvidenceSource::Session,
            format!("session://{}", session.key),
            Some(excerpt.clone()),
        ),
        excerpt,
    })
}

fn apply_budget(
    source: &str,
    candidates: Vec<AutoDreamCollectedInput>,
    remaining_budget: &mut usize,
) -> (
    Vec<AutoDreamCollectedInput>,
    AutoDreamInputSourceSummary,
    bool,
) {
    let mut included = Vec::new();
    let mut total_bytes = 0usize;
    let mut truncated = false;

    for mut item in candidates {
        if *remaining_budget == 0 {
            truncated = true;
            break;
        }

        if item.bytes > *remaining_budget {
            item.excerpt = truncate_text(&item.excerpt, *remaining_budget);
            item.bytes = item.excerpt.len();
            item.truncated = true;
            item.evidence.excerpt = Some(item.excerpt.clone());
            truncated = true;
        }

        if item.bytes == 0 {
            truncated = true;
            break;
        }

        *remaining_budget = remaining_budget.saturating_sub(item.bytes);
        total_bytes += item.bytes;
        truncated |= item.truncated;
        included.push(item);
    }

    let summary = AutoDreamInputSourceSummary {
        source: source.to_string(),
        included_items: included.len(),
        total_bytes,
        truncated,
    };
    (included, summary, truncated)
}

fn read_dir_sorted(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)
        .map_err(|source| AutoDreamError::io(dir, source))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|source| AutoDreamError::io(dir, source))?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    entries.sort();
    entries.reverse();
    Ok(entries)
}

fn truncate_text(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_string()
}

fn evidence_ref(source: EvidenceSource, uri: String, excerpt: Option<String>) -> EvidenceRef {
    EvidenceRef {
        id: format!("evidence-{}", Uuid::new_v4()),
        source,
        uri,
        excerpt,
        hash: None,
        created_at: Utc::now(),
    }
}

fn omission(source: &str, detail: impl Into<String>) -> AutoDreamInputOmission {
    AutoDreamInputOmission {
        source: source.to_string(),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::session::SessionManager;
    use agent_diva_laputa::LaputaService;
    use serde_json::Value;
    use tempfile::tempdir;

    use crate::AutoDreamStorage;

    #[test]
    fn collector_reads_sources_in_priority_order_and_truncates_total_budget() {
        let temp = tempdir().unwrap();
        seed_session(temp.path(), "chat:1", "recent session one");
        seed_session(temp.path(), "chat:2", "recent session two");
        seed_laputa(
            temp.path(),
            LaputaSectionName::MemoryMd,
            "memory authority block",
        );
        seed_capsule(temp.path(), "capsule-a.md", "capsule evidence");

        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        let collector =
            AutoDreamInputCollector::new(storage, LaputaService::open(temp.path()).unwrap())
                .with_config(AutoDreamInputCollectorConfig {
                    total_bytes_budget: 50,
                    recent_session_limit: 2,
                    recent_session_bytes: 40,
                    laputa_section_limit: 1,
                    laputa_section_bytes: 40,
                    capsule_limit: 1,
                    capsule_bytes: 40,
                    laputa_sections: vec![LaputaSectionName::MemoryMd],
                });

        let result = collector.collect("run-1").unwrap();

        assert_eq!(result.items.first().unwrap().source, SESSION_SOURCE);
        assert!(result.summary.truncated);
        assert_eq!(result.summary.included_sources[0].source, SESSION_SOURCE);
        assert_eq!(result.summary.included_sources[1].source, LAPUTA_SOURCE);
    }

    #[test]
    fn collector_records_missing_and_corrupt_sources_as_omissions() {
        let temp = tempdir().unwrap();
        let sessions_dir = temp.path().join("sessions");
        fs::create_dir_all(&sessions_dir).unwrap();
        fs::write(sessions_dir.join("bad.jsonl"), "{not json").unwrap();
        seed_laputa(
            temp.path(),
            LaputaSectionName::MemoryMd,
            "memory authority block",
        );

        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        let collector =
            AutoDreamInputCollector::new(storage, LaputaService::open(temp.path()).unwrap())
                .with_config(AutoDreamInputCollectorConfig {
                    recent_session_limit: 1,
                    laputa_section_limit: 2,
                    capsule_limit: 1,
                    laputa_sections: vec![
                        LaputaSectionName::MemoryMd,
                        LaputaSectionName::JournalReflective,
                    ],
                    ..AutoDreamInputCollectorConfig::default()
                });

        let result = collector.collect("run-1").unwrap();

        assert!(result
            .summary
            .omissions
            .iter()
            .any(|item| item.detail.contains("corrupt")));
        assert!(result
            .summary
            .omissions
            .iter()
            .any(|item| item.detail.contains("absent")));
        assert!(result
            .summary
            .omissions
            .iter()
            .any(|item| item.source == CAPSULE_SOURCE));
    }

    #[test]
    fn collector_never_writes_laputa_or_mentle_paths() {
        let temp = tempdir().unwrap();
        seed_session(temp.path(), "chat:1", "recent session one");
        seed_laputa(
            temp.path(),
            LaputaSectionName::MemoryMd,
            "memory authority block",
        );

        let laputa_path = temp.path().join(".laputa/sections/memory_md.json");
        let before = fs::read_to_string(&laputa_path).unwrap();

        let storage = AutoDreamStorage::open(temp.path()).unwrap();
        let collector =
            AutoDreamInputCollector::new(storage, LaputaService::open(temp.path()).unwrap());
        let result = collector.collect("run-1").unwrap();

        let after = fs::read_to_string(&laputa_path).unwrap();
        assert_eq!(before, after);
        assert!(!temp.path().join("MEMORY.md").exists());
        assert!(!temp.path().join(".mentle").exists());
        assert!(!result.items.is_empty());
    }

    fn seed_session(workspace: &Path, key: &str, content: &str) {
        let mut manager = SessionManager::new(workspace);
        let session = manager.get_or_create(key);
        session.add_message("user", content);
        let cloned = session.clone();
        manager.save(&cloned).unwrap();
    }

    fn seed_laputa(workspace: &Path, section: LaputaSectionName, content: &str) {
        let path = agent_diva_laputa::LaputaStorage::open(workspace)
            .unwrap()
            .paths()
            .section_file(section);
        let payload = Value::String(content.to_string());
        agent_diva_laputa::atomic_write_json(&path, &payload).unwrap();
    }

    fn seed_capsule(workspace: &Path, name: &str, content: &str) {
        let dir = workspace.join(".agent-diva/compact/capsules");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(name), content).unwrap();
    }
}
