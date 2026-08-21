use super::{
    extract_markdown_section, normalize_markdown, text::replace_markdown_section, visible_len,
    PersonaChangeRequest, PersonaDocument, PersonaError, PersonaFileState, PersonaHistoryEntry,
    PersonaHistoryRevision, PersonaInitialization, PersonaKind, PersonaRepair, PersonaRequestActor,
    PersonaRequestState, PersonaStatus, PersonaStatusView, PersonaWriteOutcome, PersonaWriteSource,
};
use crate::atomic_write;
use chrono::Utc;
use sha2::{Digest, Sha256};
use similar::{ChangeTag, TextDiff};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct PersonaService {
    root: PathBuf,
}

impl PersonaService {
    /// Open the machine-wide Persona authority below an Agent Diva config root.
    pub fn open(config_dir: impl Into<PathBuf>) -> Result<Self, PersonaError> {
        let root = config_dir.into().join("persona");
        create_dir(&root)?;
        create_dir(&root.join("history"))?;
        create_dir(&root.join("requests"))?;
        let staging = root.join(".init-staging");
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|source| PersonaError::io(&staging, source))?;
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn status(&self) -> Result<PersonaStatusView, PersonaError> {
        let requests = self.list_requests(None)?;
        let mut files = BTreeMap::new();
        for kind in PersonaKind::ALL {
            let path = self.document_path(kind);
            let exists = path.exists();
            let (valid, reason, revision, updated_at) = if !exists {
                (
                    !kind.required_for_ready(),
                    kind.required_for_ready().then(|| "missing".to_string()),
                    0,
                    None,
                )
            } else {
                match self.read_document_unchecked(kind) {
                    Ok(document) if !document.content.is_empty() => {
                        let history = self.list_history(kind)?;
                        let history_matches = history
                            .first()
                            .map(|entry| {
                                entry.revision == document.revision
                                    && entry.content_hash == document.content_hash
                            })
                            .unwrap_or(false);
                        if history_matches {
                            (true, None, document.revision, document.updated_at)
                        } else {
                            (
                                false,
                                Some("history_mismatch".to_string()),
                                document.revision,
                                document.updated_at,
                            )
                        }
                    }
                    Ok(document) => (
                        false,
                        Some("empty".to_string()),
                        document.revision,
                        document.updated_at,
                    ),
                    Err(PersonaError::InvalidContent { reason, .. }) => {
                        (false, Some(reason), self.latest_revision(kind)?, None)
                    }
                    Err(error) => return Err(error),
                }
            };
            let pending_count = requests
                .iter()
                .filter(|request| {
                    request.kind == kind && request.state == PersonaRequestState::Pending
                })
                .count();
            files.insert(
                kind,
                PersonaFileState {
                    kind,
                    file_name: kind.file_name().to_string(),
                    exists,
                    valid,
                    reason,
                    revision,
                    updated_at,
                    pending_count,
                },
            );
        }
        let required: Vec<&PersonaFileState> = PersonaKind::REQUIRED
            .iter()
            .filter_map(|kind| files.get(kind))
            .collect();
        let status = if required.iter().all(|state| !state.exists) {
            PersonaStatus::Uninitialized
        } else if required.iter().all(|state| state.exists && state.valid) {
            PersonaStatus::Ready
        } else {
            PersonaStatus::Incomplete
        };
        Ok(PersonaStatusView { status, files })
    }

    pub fn initialize(
        &self,
        initialization: PersonaInitialization,
    ) -> Result<PersonaStatusView, PersonaError> {
        if self.status()?.status != PersonaStatus::Uninitialized {
            return Err(PersonaError::Incomplete);
        }
        let mut documents = initialization.into_map();
        if let Some(user) = documents.get_mut(&PersonaKind::User) {
            *user = format!("## Preferences\n{}", normalize_markdown(user));
        }
        for (kind, content) in &documents {
            self.validate_content(*kind, content, true)?;
        }

        let staging = self.root.join(".init-staging");
        create_dir(&staging)?;
        let result: Result<(), PersonaError> = (|| {
            for (kind, content) in &documents {
                atomic_write(
                    staging.join(kind.file_name()),
                    normalize_markdown(content).as_bytes(),
                )
                .map_err(|error| PersonaError::InvalidContent {
                    kind: *kind,
                    reason: error.to_string(),
                })?;
            }
            for (kind, content) in documents {
                self.write_document_core(
                    kind,
                    &content,
                    0,
                    "user",
                    PersonaWriteSource::Init,
                    "Persona initialization",
                    None,
                )?;
            }
            Ok(())
        })();
        let _ = fs::remove_dir_all(&staging);
        result?;
        self.status()
    }

    pub fn repair(&self, repair: PersonaRepair) -> Result<PersonaStatusView, PersonaError> {
        if self.status()?.status != PersonaStatus::Incomplete {
            return Err(PersonaError::Incomplete);
        }
        let current = self.status()?;
        for (kind, mut content) in repair.documents {
            if !kind.required_for_ready() {
                return Err(PersonaError::KindForbidden(kind.to_string()));
            }
            let state = current.files.get(&kind).expect("registered Persona kind");
            if state.valid {
                return Err(PersonaError::KindForbidden(format!(
                    "repair cannot overwrite valid {kind}"
                )));
            }
            if kind == PersonaKind::User {
                content = format!("## Preferences\n{}", normalize_markdown(&content));
            }
            self.validate_content(kind, &content, true)?;
            let base_revision = self.latest_revision(kind)?;
            self.write_document_core(
                kind,
                &content,
                base_revision,
                "user",
                PersonaWriteSource::Init,
                "Persona repair",
                None,
            )?;
        }
        self.status()
    }

    pub fn get_document(&self, kind: PersonaKind) -> Result<PersonaDocument, PersonaError> {
        match self.status()?.status {
            PersonaStatus::Uninitialized => return Err(PersonaError::Uninitialized),
            PersonaStatus::Ready | PersonaStatus::Incomplete => {}
        }
        self.read_document_unchecked(kind)
    }

    pub fn save_user_document(
        &self,
        kind: PersonaKind,
        content: &str,
        base_revision: u64,
        reason: &str,
    ) -> Result<PersonaWriteOutcome, PersonaError> {
        self.require_ready()?;
        let source = if self.list_history(kind)?.iter().any(|entry| {
            self.read_history(kind, entry.revision)
                .ok()
                .is_some_and(|revision| {
                    normalize_markdown(&revision.content) == normalize_markdown(content)
                })
        }) {
            PersonaWriteSource::HistoryResave
        } else {
            PersonaWriteSource::UserDirect
        };
        self.write_document_core(kind, content, base_revision, "user", source, reason, None)
    }

    pub fn save_agent_p16(
        &self,
        kind: PersonaKind,
        content: &str,
        reason: &str,
    ) -> Result<PersonaWriteOutcome, PersonaError> {
        self.require_ready()?;
        let current = self.read_document_unchecked(kind)?;
        let proposed = match kind {
            PersonaKind::Dream | PersonaKind::Dark => content.to_string(),
            PersonaKind::User => {
                replace_markdown_section(&current.content, "Observations", content)
            }
            _ => return Err(PersonaError::KindForbidden(kind.to_string())),
        };
        self.write_document_core(
            kind,
            &proposed,
            current.revision,
            "agent",
            PersonaWriteSource::AgentP16,
            reason,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_request(
        &self,
        kind: PersonaKind,
        base_revision: u64,
        base_hash: &str,
        proposed_markdown: &str,
        actor: PersonaRequestActor,
        reason: &str,
    ) -> Result<PersonaChangeRequest, PersonaError> {
        self.require_ready()?;
        let allowed = match actor {
            PersonaRequestActor::Agent => matches!(
                kind,
                PersonaKind::Identity
                    | PersonaKind::Relationship
                    | PersonaKind::Redline
                    | PersonaKind::User
                    | PersonaKind::World
            ),
            PersonaRequestActor::Autodream => matches!(
                kind,
                PersonaKind::Identity
                    | PersonaKind::Relationship
                    | PersonaKind::User
                    | PersonaKind::World
                    | PersonaKind::Dark
            ),
        };
        if !allowed {
            return Err(PersonaError::KindForbidden(kind.to_string()));
        }
        if self
            .list_requests(Some(kind))?
            .iter()
            .any(|request| request.state == PersonaRequestState::Pending)
        {
            return Err(PersonaError::RequestExists(kind));
        }
        let current = self.read_document_unchecked(kind)?;
        if current.revision != base_revision || current.content_hash != base_hash {
            return Err(PersonaError::RevisionConflict {
                expected: base_revision,
                current: current.revision,
            });
        }
        let proposed_markdown = normalize_markdown(proposed_markdown);
        self.validate_content(kind, &proposed_markdown, true)?;
        self.validate_request_scope(kind, actor, &current.content, &proposed_markdown)?;
        let request = PersonaChangeRequest {
            id: Uuid::new_v4().to_string(),
            kind,
            base_revision,
            base_hash: base_hash.to_string(),
            proposed_markdown,
            actor,
            reason: reason.trim().to_string(),
            created_at: Utc::now(),
            state: PersonaRequestState::Pending,
            decided_at: None,
        };
        self.persist_request(&request)?;
        Ok(request)
    }

    pub fn list_requests(
        &self,
        kind: Option<PersonaKind>,
    ) -> Result<Vec<PersonaChangeRequest>, PersonaError> {
        let dir = self.root.join("requests");
        let mut requests = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|source| PersonaError::io(&dir, source))? {
            let path = entry
                .map_err(|source| PersonaError::io(&dir, source))?
                .path();
            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let bytes = fs::read(&path).map_err(|source| PersonaError::io(&path, source))?;
            let request: PersonaChangeRequest = serde_json::from_slice(&bytes)?;
            if kind.is_none() || kind == Some(request.kind) {
                requests.push(request);
            }
        }
        requests.sort_by_key(|request| std::cmp::Reverse(request.created_at));
        Ok(requests)
    }

    pub fn accept_request(&self, id: &str) -> Result<PersonaChangeRequest, PersonaError> {
        self.require_ready()?;
        let mut request = self.get_request(id)?;
        if request.state != PersonaRequestState::Pending {
            return Err(PersonaError::RequestStale(id.to_string()));
        }
        let current = self.read_document_unchecked(request.kind)?;
        if current.revision != request.base_revision || current.content_hash != request.base_hash {
            request.state = PersonaRequestState::Stale;
            request.decided_at = Some(Utc::now());
            self.persist_request(&request)?;
            return Err(PersonaError::RequestStale(id.to_string()));
        }
        self.validate_request_scope(
            request.kind,
            request.actor,
            &current.content,
            &request.proposed_markdown,
        )?;
        let source = match request.actor {
            PersonaRequestActor::Agent => PersonaWriteSource::AgentP5Accepted,
            PersonaRequestActor::Autodream => PersonaWriteSource::AutodreamP5Accepted,
        };
        self.write_document_core(
            request.kind,
            &request.proposed_markdown,
            request.base_revision,
            match request.actor {
                PersonaRequestActor::Agent => "agent",
                PersonaRequestActor::Autodream => "autodream",
            },
            source,
            &request.reason,
            Some(id),
        )?;
        request.state = PersonaRequestState::Accepted;
        request.decided_at = Some(Utc::now());
        self.persist_request(&request)?;
        Ok(request)
    }

    pub fn reject_request(&self, id: &str) -> Result<PersonaChangeRequest, PersonaError> {
        self.require_ready()?;
        let mut request = self.get_request(id)?;
        if request.state != PersonaRequestState::Pending {
            return Err(PersonaError::RequestStale(id.to_string()));
        }
        request.state = PersonaRequestState::Rejected;
        request.decided_at = Some(Utc::now());
        self.persist_request(&request)?;
        Ok(request)
    }

    pub fn list_history(
        &self,
        kind: PersonaKind,
    ) -> Result<Vec<PersonaHistoryEntry>, PersonaError> {
        let path = self.history_dir(kind).join("log.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content =
            fs::read_to_string(&path).map_err(|source| PersonaError::io(&path, source))?;
        let mut entries = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(serde_json::from_str)
            .collect::<Result<Vec<PersonaHistoryEntry>, _>>()?;
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.revision));
        Ok(entries)
    }

    pub fn read_history(
        &self,
        kind: PersonaKind,
        revision: u64,
    ) -> Result<PersonaHistoryRevision, PersonaError> {
        let entry = self
            .list_history(kind)?
            .into_iter()
            .find(|entry| entry.revision == revision)
            .ok_or_else(|| PersonaError::InvalidContent {
                kind,
                reason: format!("history revision {revision} not found"),
            })?;
        let dir = self.history_dir(kind);
        let content_path = dir.join(&entry.snapshot);
        let diff_path = dir.join(&entry.diff);
        let content = fs::read_to_string(&content_path)
            .map_err(|source| PersonaError::io(&content_path, source))?;
        let unified_diff = fs::read_to_string(&diff_path)
            .map_err(|source| PersonaError::io(&diff_path, source))?;
        Ok(PersonaHistoryRevision {
            entry,
            content,
            unified_diff,
        })
    }

    fn read_document_unchecked(&self, kind: PersonaKind) -> Result<PersonaDocument, PersonaError> {
        let path = self.document_path(kind);
        if !path.exists() {
            if kind.required_for_ready() {
                return Err(PersonaError::InvalidContent {
                    kind,
                    reason: "missing".to_string(),
                });
            }
            return Ok(PersonaDocument {
                kind,
                file_name: kind.file_name().to_string(),
                exists: false,
                valid: true,
                content: String::new(),
                revision: 0,
                content_hash: content_hash(""),
                updated_at: None,
                pending_count: self.pending_count(kind)?,
            });
        }
        let mut bytes = Vec::new();
        fs::File::open(&path)
            .and_then(|mut file| file.read_to_end(&mut bytes))
            .map_err(|source| PersonaError::io(&path, source))?;
        let raw = String::from_utf8(bytes).map_err(|_| PersonaError::InvalidContent {
            kind,
            reason: "invalid_utf8".to_string(),
        })?;
        let content = normalize_markdown(&raw);
        self.validate_content(kind, &content, kind.required_for_ready())?;
        let latest = self.list_history(kind)?.into_iter().next();
        let metadata = fs::metadata(&path).map_err(|source| PersonaError::io(&path, source))?;
        let updated_at = metadata.modified().ok().map(chrono::DateTime::<Utc>::from);
        Ok(PersonaDocument {
            kind,
            file_name: kind.file_name().to_string(),
            exists: true,
            valid: true,
            content_hash: content_hash(&content),
            content,
            revision: latest.map(|entry| entry.revision).unwrap_or(0),
            updated_at,
            pending_count: self.pending_count(kind)?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn write_document_core(
        &self,
        kind: PersonaKind,
        content: &str,
        base_revision: u64,
        actor: &str,
        source: PersonaWriteSource,
        reason: &str,
        accepting_request: Option<&str>,
    ) -> Result<PersonaWriteOutcome, PersonaError> {
        let normalized = normalize_markdown(content);
        self.validate_content(kind, &normalized, true)?;
        let current = self
            .read_document_unchecked(kind)
            .or_else(|error| match error {
                PersonaError::InvalidContent { reason, .. } if reason == "missing" => {
                    Ok(PersonaDocument {
                        kind,
                        file_name: kind.file_name().to_string(),
                        exists: false,
                        valid: false,
                        content: String::new(),
                        revision: self.latest_revision(kind)?,
                        content_hash: content_hash(""),
                        updated_at: None,
                        pending_count: 0,
                    })
                }
                other => Err(other),
            })?;
        if current.revision != base_revision {
            return Err(PersonaError::RevisionConflict {
                expected: base_revision,
                current: current.revision,
            });
        }
        if current.content_hash == content_hash(&normalized) && current.exists {
            return Ok(PersonaWriteOutcome {
                document: current,
                changed: false,
            });
        }
        let revision = current.revision + 1;
        let history_dir = self.history_dir(kind);
        create_dir(&history_dir)?;
        let snapshot_name = format!("{revision}.md");
        let diff_name = format!("{revision}.diff");
        let snapshot_path = history_dir.join(&snapshot_name);
        create_immutable(&snapshot_path, normalized.as_bytes()).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                PersonaError::RevisionConflict {
                    expected: base_revision,
                    current: self.latest_revision(kind).unwrap_or(base_revision),
                }
            } else {
                PersonaError::io(&snapshot_path, error)
            }
        })?;
        let unified_diff = unified_diff(&current.content, &normalized, kind.file_name());
        let diff_path = history_dir.join(&diff_name);
        if let Err(source) = create_immutable(&diff_path, unified_diff.as_bytes()) {
            let _ = fs::remove_file(&snapshot_path);
            return Err(PersonaError::io(&diff_path, source));
        }
        let entry = PersonaHistoryEntry {
            revision,
            content_hash: content_hash(&normalized),
            snapshot: snapshot_name,
            diff: diff_name,
            actor: actor.to_string(),
            source,
            reason: reason.trim().to_string(),
            base_revision,
            created_at: Utc::now(),
        };
        self.append_history(kind, &entry)?;
        atomic_write(self.document_path(kind), normalized.as_bytes()).map_err(|error| {
            PersonaError::InvalidContent {
                kind,
                reason: error.to_string(),
            }
        })?;
        self.stale_pending(kind, accepting_request)?;
        Ok(PersonaWriteOutcome {
            document: self.read_document_unchecked(kind)?,
            changed: true,
        })
    }

    fn require_ready(&self) -> Result<(), PersonaError> {
        match self.status()?.status {
            PersonaStatus::Ready => Ok(()),
            PersonaStatus::Uninitialized => Err(PersonaError::Uninitialized),
            PersonaStatus::Incomplete => Err(PersonaError::Incomplete),
        }
    }

    fn validate_content(
        &self,
        kind: PersonaKind,
        content: &str,
        require_nonempty: bool,
    ) -> Result<(), PersonaError> {
        let normalized = normalize_markdown(content);
        if require_nonempty && normalized.is_empty() {
            return Err(PersonaError::InvalidContent {
                kind,
                reason: "empty".to_string(),
            });
        }
        if visible_len(&normalized) > kind.content_limit() {
            return Err(PersonaError::CapExceeded {
                kind,
                limit: kind.content_limit(),
            });
        }
        Ok(())
    }

    fn validate_request_scope(
        &self,
        kind: PersonaKind,
        actor: PersonaRequestActor,
        current: &str,
        proposed: &str,
    ) -> Result<(), PersonaError> {
        if kind == PersonaKind::User {
            let current_observations = extract_markdown_section(current, "Observations");
            let proposed_observations = extract_markdown_section(proposed, "Observations");
            if actor == PersonaRequestActor::Agent && current_observations != proposed_observations
            {
                return Err(PersonaError::KindForbidden(
                    "agent P5 request cannot modify USER Observations".to_string(),
                ));
            }
            if actor == PersonaRequestActor::Autodream {
                let current_preferences = user_preferences(current);
                let proposed_preferences = user_preferences(proposed);
                if current_preferences != proposed_preferences {
                    return Err(PersonaError::KindForbidden(
                        "AutoDream cannot modify USER Preferences".to_string(),
                    ));
                }
            }
        }
        if kind == PersonaKind::World && !world_user_content_preserved(current, proposed) {
            return Err(PersonaError::WorldProtectedClaim);
        }
        if kind == PersonaKind::World && !world_entry_gate_allows(actor, current, proposed) {
            return Err(PersonaError::WorldEntryGate);
        }
        Ok(())
    }

    fn get_request(&self, id: &str) -> Result<PersonaChangeRequest, PersonaError> {
        let path = self.request_path(id);
        if !path.exists() {
            return Err(PersonaError::RequestNotFound(id.to_string()));
        }
        let bytes = fs::read(&path).map_err(|source| PersonaError::io(&path, source))?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn persist_request(&self, request: &PersonaChangeRequest) -> Result<(), PersonaError> {
        let path = self.request_path(&request.id);
        let bytes = serde_json::to_vec_pretty(request)?;
        atomic_write(&path, &bytes).map_err(|error| PersonaError::InvalidContent {
            kind: request.kind,
            reason: error.to_string(),
        })
    }

    fn stale_pending(
        &self,
        kind: PersonaKind,
        accepting_request: Option<&str>,
    ) -> Result<(), PersonaError> {
        for mut request in self.list_requests(Some(kind))? {
            if request.state == PersonaRequestState::Pending
                && accepting_request != Some(request.id.as_str())
            {
                request.state = PersonaRequestState::Stale;
                request.decided_at = Some(Utc::now());
                self.persist_request(&request)?;
            }
        }
        Ok(())
    }

    fn pending_count(&self, kind: PersonaKind) -> Result<usize, PersonaError> {
        Ok(self
            .list_requests(Some(kind))?
            .iter()
            .filter(|request| request.state == PersonaRequestState::Pending)
            .count())
    }

    fn latest_revision(&self, kind: PersonaKind) -> Result<u64, PersonaError> {
        Ok(self
            .list_history(kind)?
            .first()
            .map(|entry| entry.revision)
            .unwrap_or(0))
    }

    fn append_history(
        &self,
        kind: PersonaKind,
        entry: &PersonaHistoryEntry,
    ) -> Result<(), PersonaError> {
        let path = self.history_dir(kind).join("log.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| PersonaError::io(&path, source))?;
        serde_json::to_writer(&mut file, entry)?;
        file.write_all(b"\n")
            .and_then(|_| file.sync_all())
            .map_err(|source| PersonaError::io(&path, source))
    }

    fn document_path(&self, kind: PersonaKind) -> PathBuf {
        self.root.join(kind.file_name())
    }

    fn history_dir(&self, kind: PersonaKind) -> PathBuf {
        self.root.join("history").join(kind.file_name())
    }

    fn request_path(&self, id: &str) -> PathBuf {
        self.root.join("requests").join(format!("{id}.json"))
    }
}

fn create_dir(path: &Path) -> Result<(), PersonaError> {
    fs::create_dir_all(path).map_err(|source| PersonaError::io(path, source))
}

fn create_immutable(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn content_hash(content: &str) -> String {
    let digest = Sha256::digest(normalize_markdown(content).as_bytes());
    format!("sha256:{digest:x}")
}

fn unified_diff(before: &str, after: &str, file_name: &str) -> String {
    let diff = TextDiff::from_lines(before, after);
    let mut output = format!("--- a/{file_name}\n+++ b/{file_name}\n");
    for operation in diff.ops() {
        for change in diff.iter_changes(operation) {
            let prefix = match change.tag() {
                ChangeTag::Delete => '-',
                ChangeTag::Insert => '+',
                ChangeTag::Equal => ' ',
            };
            output.push(prefix);
            output.push_str(change.value());
            if !change.value().ends_with('\n') {
                output.push('\n');
            }
        }
    }
    output
}

fn user_preferences(content: &str) -> String {
    extract_markdown_section(content, "Preferences")
        .map(normalize_markdown)
        .unwrap_or_else(|| normalize_markdown(content))
}

fn world_user_content_preserved(current: &str, proposed: &str) -> bool {
    let protected = protected_world_segments(current);
    protected
        .iter()
        .all(|segment| proposed.contains(segment.trim()))
}

fn protected_world_segments(content: &str) -> Vec<String> {
    let mut protected = Vec::new();
    let mut current_block = String::new();
    for line in normalize_markdown(content).lines() {
        if line.starts_with("## [") && !current_block.is_empty() {
            if block_is_user_protected(&current_block) || !current_block.starts_with("## [") {
                protected.push(current_block.trim().to_string());
            }
            current_block.clear();
        }
        if !current_block.is_empty() {
            current_block.push('\n');
        }
        current_block.push_str(line);
    }
    if !current_block.is_empty()
        && (block_is_user_protected(&current_block) || !current_block.starts_with("## ["))
    {
        protected.push(current_block.trim().to_string());
    }
    protected
}

fn block_is_user_protected(block: &str) -> bool {
    block
        .lines()
        .any(|line| line.trim() == "- status: confirmed")
        && block.lines().any(|line| line.trim() == "- source: user")
}

fn world_entry_gate_allows(actor: PersonaRequestActor, current: &str, proposed: &str) -> bool {
    let (current_prose, current_claims) = split_world_content(current);
    let (proposed_prose, proposed_claims) = split_world_content(proposed);
    if proposed_prose
        .iter()
        .any(|segment| !current_prose.contains(segment))
    {
        return false;
    }
    if proposed_claims
        .values()
        .any(|block| !valid_world_claim(block))
    {
        return false;
    }
    if actor == PersonaRequestActor::Autodream {
        return current_claims.iter().all(|(identity, block)| {
            proposed_claims
                .get(identity)
                .is_some_and(|next| next == block)
        });
    }
    true
}

fn split_world_content(content: &str) -> (Vec<String>, BTreeMap<String, String>) {
    let mut prose = Vec::new();
    let mut claims = BTreeMap::new();
    let mut segment = String::new();
    for line in normalize_markdown(content).lines() {
        if line.starts_with("## [") && !segment.is_empty() {
            store_world_segment(&mut prose, &mut claims, &segment);
            segment.clear();
        }
        if !segment.is_empty() {
            segment.push('\n');
        }
        segment.push_str(line);
    }
    if !segment.is_empty() {
        store_world_segment(&mut prose, &mut claims, &segment);
    }
    (prose, claims)
}

fn store_world_segment(
    prose: &mut Vec<String>,
    claims: &mut BTreeMap<String, String>,
    segment: &str,
) {
    let normalized = segment.trim().to_string();
    if let Some(identity) = world_claim_identity(&normalized) {
        claims.insert(identity, normalized);
    } else if !normalized.is_empty() {
        prose.push(normalized);
    }
}

fn world_claim_identity(block: &str) -> Option<String> {
    let heading = block.lines().next()?.strip_prefix("## [")?;
    let (domain, title) = heading.split_once("] ")?;
    let domain = domain.trim();
    let title = title.trim();
    if domain.is_empty() || title.is_empty() {
        return None;
    }
    Some(format!(
        "{}:{}",
        domain.to_ascii_lowercase(),
        title.to_ascii_lowercase()
    ))
}

fn valid_world_claim(block: &str) -> bool {
    world_claim_identity(block).is_some()
        && block
            .lines()
            .any(|line| line.trim().starts_with("- status: "))
        && block
            .lines()
            .any(|line| line.trim().starts_with("- source: "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn initialized() -> (TempDir, PersonaService) {
        let temp = TempDir::new().unwrap();
        let service = PersonaService::open(temp.path()).unwrap();
        service
            .initialize(PersonaInitialization {
                identity: "# Identity\nDiva".into(),
                relationship: "# Relationship\nPartner".into(),
                redline: "# Redline\nAsk first".into(),
                user: "Concise".into(),
                world: "# World\nLocal project".into(),
            })
            .unwrap();
        (temp, service)
    }

    #[test]
    fn open_creates_only_directories() {
        let temp = TempDir::new().unwrap();
        let service = PersonaService::open(temp.path()).unwrap();
        assert_eq!(
            service.status().unwrap().status,
            PersonaStatus::Uninitialized
        );
        for kind in PersonaKind::ALL {
            assert!(!service.root().join(kind.file_name()).exists());
        }
    }

    #[test]
    fn initialization_creates_five_heads_and_history_only() {
        let (_temp, service) = initialized();
        assert_eq!(service.status().unwrap().status, PersonaStatus::Ready);
        assert!(!service.root().join("DREAM.MD").exists());
        assert!(!service.root().join("DARK.MD").exists());
        let user = service.get_document(PersonaKind::User).unwrap();
        assert_eq!(user.revision, 1);
        assert!(user.content.starts_with("## Preferences\n"));
        assert_eq!(service.list_history(PersonaKind::User).unwrap().len(), 1);
    }

    #[test]
    fn no_op_and_conflict_preserve_revision() {
        let (_temp, service) = initialized();
        let current = service.get_document(PersonaKind::Identity).unwrap();
        let no_op = service
            .save_user_document(
                PersonaKind::Identity,
                &current.content,
                current.revision,
                "same",
            )
            .unwrap();
        assert!(!no_op.changed);
        assert!(matches!(
            service.save_user_document(PersonaKind::Identity, "changed", 0, "stale"),
            Err(PersonaError::RevisionConflict { .. })
        ));
    }

    #[test]
    fn direct_write_stales_request_and_accept_writes_head() {
        let (_temp, service) = initialized();
        let current = service.get_document(PersonaKind::Identity).unwrap();
        let request = service
            .create_request(
                PersonaKind::Identity,
                current.revision,
                &current.content_hash,
                "# Identity\nNova",
                PersonaRequestActor::Agent,
                "rename",
            )
            .unwrap();
        let accepted = service.accept_request(&request.id).unwrap();
        assert_eq!(accepted.state, PersonaRequestState::Accepted);
        assert_eq!(
            service.get_document(PersonaKind::Identity).unwrap().content,
            "# Identity\nNova"
        );
    }

    #[test]
    fn p16_user_write_preserves_preferences() {
        let (_temp, service) = initialized();
        service
            .save_agent_p16(PersonaKind::User, "Notices careful wording", "observe")
            .unwrap();
        let user = service.get_document(PersonaKind::User).unwrap();
        assert!(user.content.contains("## Preferences\nConcise"));
        assert!(user
            .content
            .contains("## Observations\nNotices careful wording"));
    }

    #[test]
    fn world_request_cannot_remove_user_protected_content() {
        let (_temp, service) = initialized();
        let current = service.get_document(PersonaKind::World).unwrap();
        assert!(matches!(
            service.create_request(
                PersonaKind::World,
                current.revision,
                &current.content_hash,
                "## [work] New\n- status: tentative",
                PersonaRequestActor::Agent,
                "replace",
            ),
            Err(PersonaError::WorldProtectedClaim)
        ));
    }

    #[test]
    fn world_r6_rejects_unstructured_agent_additions() {
        let (_temp, service) = initialized();
        let current = service.get_document(PersonaKind::World).unwrap();
        let proposed = format!("{}\n\nA vague new world belief", current.content);
        assert!(matches!(
            service.create_request(
                PersonaKind::World,
                current.revision,
                &current.content_hash,
                &proposed,
                PersonaRequestActor::Agent,
                "test",
            ),
            Err(PersonaError::WorldEntryGate)
        ));
    }

    #[test]
    fn world_r6_accepts_bounded_reviewable_agent_claim() {
        let (_temp, service) = initialized();
        let current = service.get_document(PersonaKind::World).unwrap();
        let proposed = format!(
            "{}\n\n## [project] Local repository\n- status: observed\n- source: agent\n- action: preserve unrelated changes",
            current.content
        );
        let request = service
            .create_request(
                PersonaKind::World,
                current.revision,
                &current.content_hash,
                &proposed,
                PersonaRequestActor::Agent,
                "test",
            )
            .unwrap();
        assert_eq!(request.state, PersonaRequestState::Pending);
    }
}
