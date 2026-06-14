use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use agent_diva_core::evolution::{
    AuditEvent, AuditEventKind, ChangelogAction, ChangelogRecord, EvolutionProposal,
    LaputaSectionName, ProposalState,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;

use crate::{
    atomic_write_json, LaputaError, LaputaLock, LaputaStorage, LockOptions, ProposalFilter,
    ProposalRepository, Result,
};

const SNAPSHOT_SCHEMA_VERSION: &str = "1.0.0";
const ROLLBACK_WINDOW: Duration = Duration::from_secs(31 * 24 * 60 * 60);
const EVENT_CHANNEL_CAPACITY: usize = 256;

/// Stable Laputa service API used by Rust callers, manager routes, and Tauri commands.
#[derive(Clone, Debug)]
pub struct LaputaService {
    storage: LaputaStorage,
    proposals: ProposalRepository,
    events: LaputaEventBus,
}

impl LaputaService {
    pub fn open(workspace_root: impl Into<PathBuf>) -> Result<Self> {
        let storage = LaputaStorage::open(workspace_root)?;
        Ok(Self::from_storage(storage))
    }

    pub fn from_storage(storage: LaputaStorage) -> Self {
        Self {
            proposals: ProposalRepository::new(storage.clone()),
            storage,
            events: LaputaEventBus::default(),
        }
    }

    pub fn create_proposal(&self, proposal: EvolutionProposal) -> Result<EvolutionProposal> {
        let proposal = self.proposals.create_proposal(proposal)?;
        self.record_proposal_event(&proposal)?;
        Ok(proposal)
    }

    pub fn get_proposal(&self, id: &str) -> Result<EvolutionProposal> {
        self.proposals.get_proposal(id)
    }

    pub fn list_proposals(&self, filter: ProposalFilter) -> Result<Vec<EvolutionProposal>> {
        self.proposals.list_proposals(filter)
    }

    pub fn edit_proposal(&self, id: &str, edit: crate::ProposalEdit) -> Result<EvolutionProposal> {
        let proposal = self.proposals.edit_proposal(id, edit)?;
        self.record_proposal_event(&proposal)?;
        Ok(proposal)
    }

    pub fn transition_proposal(
        &self,
        id: &str,
        to: ProposalState,
        updated_at: DateTime<Utc>,
    ) -> Result<EvolutionProposal> {
        let proposal = self.proposals.transition_proposal(id, to, updated_at)?;
        self.record_proposal_event(&proposal)?;
        Ok(proposal)
    }

    pub fn apply_proposal(
        &self,
        id: &str,
        actor: impl Into<String>,
        applied_at: DateTime<Utc>,
    ) -> Result<crate::ApplyOutcome> {
        let outcome = self.proposals.apply_proposal(id, actor, applied_at)?;
        self.record_proposal_event(&outcome.proposal)?;
        self.record_changelog_event(&outcome.changelog)?;
        Ok(outcome)
    }

    pub fn read_snapshot(&self, since: Option<DateTime<Utc>>) -> Result<LaputaSnapshot> {
        let mut sections = BTreeMap::new();
        let mut updated_at = None;
        let mut changed_sections = Vec::new();

        for name in LaputaSectionName::all_v1() {
            let section = self.read_section(name.clone())?;
            if let Some(last_modified) = section.last_modified {
                updated_at = Some(updated_at.map_or(last_modified, |current| {
                    if last_modified > current {
                        last_modified
                    } else {
                        current
                    }
                }));
                if since.map_or(true, |since| last_modified >= since) {
                    changed_sections.push(section.name.as_str().to_string());
                }
            } else if since.is_none() {
                changed_sections.push(section.name.as_str().to_string());
            }
            sections.insert(section.name.as_str().to_string(), section);
        }

        Ok(LaputaSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION.to_string(),
            sections,
            changed_sections,
            updated_at,
            server_time: Utc::now(),
        })
    }

    pub fn read_section(&self, name: LaputaSectionName) -> Result<LaputaSection> {
        let path = self.storage.paths().section_file(name.clone());
        let last_modified = file_modified_at(&path)?;
        let content = match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| json!(content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => serde_json::Value::Null,
            Err(source) => return Err(LaputaError::io(path, source)),
        };

        Ok(LaputaSection {
            name: name.clone(),
            status: section_status(&name),
            content,
            metadata: json!({
                "status": section_status(&name),
                "content_type": section_content_type(&name),
            }),
            last_modified,
            version: SNAPSHOT_SCHEMA_VERSION.to_string(),
        })
    }

    pub fn list_changelog(&self, filter: ChangelogFilter) -> Result<ChangelogPage> {
        let mut records =
            read_json_records::<ChangelogRecord>(&self.storage.paths().changelog_dir())?;
        records.retain(|record| filter.matches(record));
        records.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });

        let total = records.len();
        let page = filter.page.unwrap_or(1).max(1);
        let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
        let start = (page - 1).saturating_mul(page_size);
        let items = records
            .into_iter()
            .skip(start)
            .take(page_size)
            .collect::<Vec<_>>();
        let has_more = start + items.len() < total;

        Ok(ChangelogPage {
            items,
            total,
            page,
            page_size,
            has_more,
        })
    }

    pub fn get_changelog(&self, id: &str) -> Result<ChangelogRecord> {
        read_json_file(
            self.storage
                .paths()
                .changelog_dir()
                .join(format!("{id}.json")),
        )
        .map_err(|error| match error {
            LaputaError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
                LaputaError::ChangelogNotFound { id: id.to_string() }
            }
            other => other,
        })
    }

    pub fn rollback_changelog(
        &self,
        id: &str,
        request: RollbackChangelogRequest,
        actor: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<RollbackOutcome> {
        let _guard = LaputaLock::acquire(
            self.storage.paths().lock_file("rollback"),
            LockOptions::default(),
        )?;
        let actor = actor.into();
        let mut original = self.get_changelog(id)?;
        if original.action != ChangelogAction::Apply {
            return Err(LaputaError::RollbackIneligible {
                id: original.id,
                action: original.action,
            });
        }
        if now - original.created_at > chrono::Duration::seconds(ROLLBACK_WINDOW.as_secs() as i64) {
            return Err(LaputaError::RollbackExpired { id: id.to_string() });
        }

        let section_path = self
            .storage
            .paths()
            .section_file(original.target_section.clone());
        let current = fs::read_to_string(&section_path).unwrap_or_default();
        if let Some(expected) = request.expected_current.as_ref() {
            if expected != &current {
                let error = LaputaError::RollbackConflict {
                    id: id.to_string(),
                    reason: "expected_current does not match current section content".to_string(),
                };
                self.record_error_event(
                    &error,
                    Some(original.target_section.clone()),
                    original.proposal_id.clone(),
                )?;
                return Err(error);
            }
        }

        crate::atomic_write(&section_path, original.before.as_bytes())?;

        let rollback_id = format!("rollback-{}-{}", original.id, now.timestamp_millis());
        let audit_id = format!("audit-{}-{}", rollback_id, now.timestamp_millis());
        let changelog = ChangelogRecord {
            id: rollback_id,
            action: ChangelogAction::Rollback,
            target_section: original.target_section.clone(),
            before: current,
            after: original.before.clone(),
            diff: original.before.clone(),
            proposal_id: original.proposal_id.clone(),
            audit_event_id: Some(audit_id.clone()),
            reverted: false,
            stale: false,
            created_at: now,
            applied_by: actor.clone(),
        };
        let audit_event = AuditEvent {
            id: audit_id,
            kind: AuditEventKind::RollbackApplied,
            actor,
            proposal_id: original.proposal_id.clone(),
            target_section: Some(original.target_section.clone()),
            message: format!("rolled back changelog {}", original.id),
            created_at: now,
        };

        atomic_write_json(
            self.storage
                .paths()
                .changelog_dir()
                .join(format!("{}.json", changelog.id)),
            &changelog,
        )?;
        atomic_write_json(
            self.storage
                .paths()
                .audit_dir()
                .join(format!("{}.json", audit_event.id)),
            &audit_event,
        )?;
        original.reverted = true;
        atomic_write_json(
            self.storage
                .paths()
                .changelog_dir()
                .join(format!("{}.json", original.id)),
            &original,
        )?;

        if let Some(proposal_id) = original.proposal_id.as_deref() {
            if let Ok(proposal) =
                self.proposals
                    .transition_proposal(proposal_id, ProposalState::Reverted, now)
            {
                self.record_proposal_event(&proposal)?;
            }
        }
        self.record_changelog_event(&changelog)?;

        Ok(RollbackOutcome {
            changelog,
            audit_event,
        })
    }

    pub fn poll_events(
        &self,
        kind: Option<LaputaEventKind>,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<LaputaEvent>> {
        let mut events = read_jsonl_events(&self.storage.paths().events_jsonl())?;
        events.retain(|event| {
            kind.as_ref().map_or(true, |kind| event.kind == *kind)
                && since.map_or(true, |since| event.timestamp >= since)
        });
        Ok(events)
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<LaputaEvent> {
        self.events.sender.subscribe()
    }

    fn record_proposal_event(&self, proposal: &EvolutionProposal) -> Result<()> {
        self.record_event(LaputaEvent {
            event_id: event_id("proposal", &proposal.id, proposal.updated_at),
            kind: LaputaEventKind::Proposal,
            proposal_id: Some(proposal.id.clone()),
            proposal_type: Some(proposal.proposal_type.to_string()),
            target_section: Some(proposal.target_section.as_str().to_string()),
            status: Some(serde_variant(&proposal.state)?),
            changelog_id: None,
            action: None,
            error_type: None,
            conflict_reason: None,
            timestamp: proposal.updated_at,
        })
    }

    fn record_changelog_event(&self, changelog: &ChangelogRecord) -> Result<()> {
        self.record_event(LaputaEvent {
            event_id: event_id("changelog", &changelog.id, changelog.created_at),
            kind: LaputaEventKind::Changelog,
            proposal_id: changelog.proposal_id.clone(),
            proposal_type: None,
            target_section: Some(changelog.target_section.as_str().to_string()),
            status: None,
            changelog_id: Some(changelog.id.clone()),
            action: Some(serde_variant(&changelog.action)?),
            error_type: None,
            conflict_reason: None,
            timestamp: changelog.created_at,
        })
    }

    fn record_error_event(
        &self,
        error: &LaputaError,
        target_section: Option<LaputaSectionName>,
        proposal_id: Option<String>,
    ) -> Result<()> {
        self.record_event(LaputaEvent {
            event_id: event_id("error", "laputa", Utc::now()),
            kind: LaputaEventKind::Error,
            proposal_id,
            proposal_type: None,
            target_section: target_section.map(|section| section.as_str().to_string()),
            status: Some("needs_attention".to_string()),
            changelog_id: None,
            action: None,
            error_type: Some(error_name(error).to_string()),
            conflict_reason: Some(error.to_string()),
            timestamp: Utc::now(),
        })
    }

    fn record_event(&self, event: LaputaEvent) -> Result<()> {
        append_jsonl(self.storage.paths().events_jsonl(), &event)?;
        self.events.record(event);
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct LaputaEventBus {
    sender: broadcast::Sender<LaputaEvent>,
    buffer: Arc<Mutex<Vec<LaputaEvent>>>,
}

impl Default for LaputaEventBus {
    fn default() -> Self {
        let (sender, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            sender,
            buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl LaputaEventBus {
    fn record(&self, event: LaputaEvent) {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push(event.clone());
            if buffer.len() > 1000 {
                let overflow = buffer.len() - 1000;
                buffer.drain(0..overflow);
            }
        }
        let _ = self.sender.send(event);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SectionStatus {
    Owned,
    Tbd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaputaSection {
    pub name: LaputaSectionName,
    pub status: SectionStatus,
    pub content: serde_json::Value,
    pub metadata: serde_json::Value,
    pub last_modified: Option<DateTime<Utc>>,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaputaSnapshot {
    pub schema_version: String,
    pub sections: BTreeMap<String, LaputaSection>,
    pub changed_sections: Vec<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub server_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangelogFilter {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub since: Option<DateTime<Utc>>,
    pub until: Option<DateTime<Utc>>,
    pub target_section: Option<LaputaSectionName>,
    pub action: Option<ChangelogAction>,
    pub proposal_id: Option<String>,
}

impl ChangelogFilter {
    fn matches(&self, record: &ChangelogRecord) -> bool {
        self.since.map_or(true, |since| record.created_at >= since)
            && self.until.map_or(true, |until| record.created_at <= until)
            && self
                .target_section
                .as_ref()
                .map_or(true, |section| record.target_section == *section)
            && self
                .action
                .as_ref()
                .map_or(true, |action| record.action == *action)
            && self.proposal_id.as_ref().map_or(true, |proposal_id| {
                record.proposal_id.as_ref() == Some(proposal_id)
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangelogPage {
    pub items: Vec<ChangelogRecord>,
    pub total: usize,
    pub page: usize,
    pub page_size: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackChangelogRequest {
    pub reason: String,
    pub expected_current: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackOutcome {
    pub changelog: ChangelogRecord,
    pub audit_event: AuditEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaputaEventKind {
    Proposal,
    Changelog,
    Error,
    BufferOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaputaEvent {
    pub event_id: String,
    pub kind: LaputaEventKind,
    pub proposal_id: Option<String>,
    pub proposal_type: Option<String>,
    pub target_section: Option<String>,
    pub status: Option<String>,
    pub changelog_id: Option<String>,
    pub action: Option<String>,
    pub error_type: Option<String>,
    pub conflict_reason: Option<String>,
    pub timestamp: DateTime<Utc>,
}

fn section_status(section: &LaputaSectionName) -> SectionStatus {
    match section {
        LaputaSectionName::JournalReflective
        | LaputaSectionName::ProposalInbox
        | LaputaSectionName::Changelog
        | LaputaSectionName::ReportIndexes
        | LaputaSectionName::AaakSummaries => SectionStatus::Tbd,
        _ => SectionStatus::Owned,
    }
}

fn section_content_type(section: &LaputaSectionName) -> &'static str {
    match section_status(section) {
        SectionStatus::Owned => "json",
        SectionStatus::Tbd => "tbd",
    }
}

fn file_modified_at(path: &Path) -> Result<Option<DateTime<Utc>>> {
    match fs::metadata(path) {
        Ok(metadata) => metadata
            .modified()
            .map(|time| Some(DateTime::<Utc>::from(time)))
            .map_err(|source| LaputaError::io(path, source)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(LaputaError::io(path, source)),
    }
}

fn read_json_file<T>(path: impl Into<PathBuf>) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let path = path.into();
    let bytes = fs::read(&path).map_err(|source| LaputaError::io(&path, source))?;
    serde_json::from_slice(&bytes).map_err(LaputaError::from)
}

fn read_json_records<T>(dir: &Path) -> Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let mut records = Vec::new();
    for entry in fs::read_dir(dir).map_err(|source| LaputaError::io(dir, source))? {
        let entry = entry.map_err(|source| LaputaError::io(dir, source))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            records.push(read_json_file(path)?);
        }
    }
    Ok(records)
}

fn append_jsonl(path: PathBuf, event: &LaputaEvent) -> Result<()> {
    let mut events = read_jsonl_events(&path)?;
    events.push(event.clone());
    let body = events
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?
        .join("\n");
    let body = if body.is_empty() {
        body
    } else {
        format!("{body}\n")
    };
    crate::atomic_write(path, body.as_bytes())
}

fn read_jsonl_events(path: &Path) -> Result<Vec<LaputaEvent>> {
    match fs::read_to_string(path) {
        Ok(content) => content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).map_err(LaputaError::from))
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(source) => Err(LaputaError::io(path, source)),
    }
}

fn serde_variant<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    match serde_json::to_value(value)? {
        serde_json::Value::String(value) => Ok(value),
        other => Ok(other.to_string()),
    }
}

fn event_id(kind: &str, id: &str, timestamp: DateTime<Utc>) -> String {
    format!("{kind}-{id}-{}", timestamp.timestamp_millis())
}

fn error_name(error: &LaputaError) -> &'static str {
    match error {
        LaputaError::RollbackConflict { .. } => "rollback_conflict",
        LaputaError::UnresolvedConflict { .. } => "unresolved_conflict",
        LaputaError::SchemaMismatch { .. } => "schema_mismatch",
        LaputaError::UnauthorizedTarget { .. } => "unauthorized_target",
        LaputaError::LockTimeout { .. } => "lock_timeout",
        _ => "laputa_error",
    }
}
