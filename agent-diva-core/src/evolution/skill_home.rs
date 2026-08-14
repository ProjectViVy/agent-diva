//! Machine-wide Skill authority and review-request lifecycle.

use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const SKILL_FILE: &str = "SKILL.md";
const REQUESTS_DIR: &str = "requests";
const HISTORY_DIR: &str = "history";
const ZERO_HASH: &str = "0";

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Machine-wide monotonic generation used by every Session skill cache.
static MACHINE_SKILL_EPOCH: AtomicU64 = AtomicU64::new(1);

/// Current process-wide Skill generation.
pub fn machine_skill_epoch() -> u64 {
    MACHINE_SKILL_EPOCH.load(Ordering::Acquire)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillSource {
    Home,
    Builtin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillSummary {
    pub slug: String,
    pub description: String,
    pub source: SkillSource,
    pub enabled: bool,
    pub always: bool,
    pub available: bool,
    pub content_hash: String,
    pub updated_at: DateTime<Utc>,
    pub can_hard_delete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDocument {
    #[serde(flatten)]
    pub summary: SkillSummary,
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillHistoryEntry {
    pub revision: u64,
    pub content_hash: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillHistoryDocument {
    pub slug: String,
    pub revision: u64,
    pub content_hash: String,
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillProposalStatus {
    Pending,
    Accepted,
    Rejected,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillProposalSource {
    Autodream,
    Distill,
    UserRequest,
}

impl SkillProposalSource {
    fn is_automatic(&self) -> bool {
        matches!(self, Self::Autodream | Self::Distill)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvidence {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actmem_pointer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autodream_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
}

impl SkillEvidence {
    fn is_valid(&self) -> bool {
        [
            &self.session_key,
            &self.actmem_pointer,
            &self.autodream_run_id,
        ]
        .into_iter()
        .any(|value| {
            value
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposal {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub proposed_markdown: String,
    pub evidence: Vec<SkillEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attestation: Option<String>,
    pub base_hash: String,
    pub source: SkillProposalSource,
    pub reason: String,
    pub status: SkillProposalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSkillProposal {
    pub slug: String,
    pub title: String,
    pub proposed_markdown: String,
    #[serde(default)]
    pub evidence: Vec<SkillEvidence>,
    #[serde(default)]
    pub attestation: Option<String>,
    pub base_hash: String,
    pub source: SkillProposalSource,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillWriteOutcome {
    pub document: SkillDocument,
    pub changed: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SkillHomeError {
    #[error("invalid skill slug: {0}")]
    SlugInvalid(String),
    #[error("skill not found: {0}")]
    NotFound(String),
    #[error("skill already exists: {0}")]
    AlreadyExists(String),
    #[error("skill content hash conflict")]
    HashConflict,
    #[error("a pending skill request already exists for {0}")]
    RequestExists(String),
    #[error("skill request is stale")]
    RequestStale,
    #[error("skill evidence is required")]
    EvidenceRequired,
    #[error("skill content contains a secret-like value")]
    SecretRejected,
    #[error("invalid SKILL.md: {0}")]
    InvalidMarkdown(String),
    #[error("invalid skill package path: {0}")]
    InvalidPackagePath(String),
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("serialization error: {0}")]
    Serialization(String),
}

impl SkillHomeError {
    /// Stable error code shared by HTTP and Tauri envelopes.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::SlugInvalid(_) => "skill_slug_invalid",
            Self::HashConflict => "skill_hash_conflict",
            Self::RequestExists(_) => "skill_request_exists",
            Self::RequestStale => "skill_request_stale",
            Self::EvidenceRequired => "skill_evidence_required",
            Self::SecretRejected => "skill_secret_rejected",
            Self::NotFound(_) => "skill_not_found",
            Self::AlreadyExists(_) => "skill_already_exists",
            Self::InvalidMarkdown(_) | Self::InvalidPackagePath(_) => "skill_invalid",
            Self::Io { .. } | Self::Serialization(_) => "skill_internal_error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkillHome {
    inner: Arc<SkillHomeInner>,
}

#[derive(Debug)]
struct SkillHomeInner {
    root: PathBuf,
    builtin_root: PathBuf,
    write_lock: Mutex<()>,
}

#[derive(Debug)]
struct ParsedSkill {
    description: String,
    enabled: bool,
    always: bool,
    body: String,
    yaml: Mapping,
}

impl SkillHome {
    /// Create the machine-wide authority at `{config_dir}/skills`.
    pub fn new(config_dir: impl AsRef<Path>, builtin_root: impl Into<PathBuf>) -> Self {
        Self {
            inner: Arc::new(SkillHomeInner {
                root: config_dir.as_ref().join("skills"),
                builtin_root: builtin_root.into(),
                write_lock: Mutex::new(()),
            }),
        }
    }

    pub fn root(&self) -> &Path {
        &self.inner.root
    }

    pub fn builtin_root(&self) -> &Path {
        &self.inner.builtin_root
    }

    pub fn list(&self) -> Result<Vec<SkillSummary>, SkillHomeError> {
        let mut slugs = BTreeMap::<String, SkillSource>::new();
        self.collect_slugs(&self.inner.builtin_root, SkillSource::Builtin, &mut slugs)?;
        self.collect_slugs(&self.inner.root, SkillSource::Home, &mut slugs)?;
        slugs
            .into_keys()
            .map(|slug| self.read(&slug).map(|document| document.summary))
            .collect()
    }

    pub fn read(&self, slug: &str) -> Result<SkillDocument, SkillHomeError> {
        validate_skill_slug(slug)?;
        let (path, source) = self.resolve_path(slug)?;
        let markdown = read_string(&path)?;
        let parsed = parse_skill_markdown(&markdown)?;
        let metadata = fs::metadata(&path).map_err(|source| io_error(&path, source))?;
        let updated_at = metadata
            .modified()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());
        Ok(SkillDocument {
            summary: SkillSummary {
                slug: slug.to_string(),
                description: parsed.description,
                source: source.clone(),
                enabled: parsed.enabled,
                always: parsed.always,
                available: true,
                content_hash: content_hash(&markdown),
                updated_at,
                can_hard_delete: source == SkillSource::Home,
            },
            markdown,
        })
    }

    /// Read only an enabled skill. Disabled Home overrides never fall back.
    pub fn read_enabled(&self, slug: &str) -> Result<SkillDocument, SkillHomeError> {
        let document = self.read(slug)?;
        if document.summary.enabled {
            Ok(document)
        } else {
            Err(SkillHomeError::NotFound(slug.to_string()))
        }
    }

    /// CAS update an existing effective Skill. Builtins become Home overrides.
    pub fn update(
        &self,
        slug: &str,
        markdown: &str,
        base_hash: &str,
    ) -> Result<SkillWriteOutcome, SkillHomeError> {
        validate_skill_slug(slug)?;
        let _guard = self.inner.write_lock.lock();
        let current = self.read(slug)?;
        if current.summary.content_hash != base_hash {
            return Err(SkillHomeError::HashConflict);
        }
        reject_secrets(markdown)?;
        let normalized = normalize_skill_markdown(markdown, None)?;
        if normalized == current.markdown {
            return Ok(SkillWriteOutcome {
                document: current,
                changed: false,
            });
        }
        self.write_head_and_history(slug, &normalized)?;
        self.mark_pending_stale(slug)?;
        bump_machine_epoch();
        Ok(SkillWriteOutcome {
            document: self.read(slug)?,
            changed: true,
        })
    }

    pub fn disable(
        &self,
        slug: &str,
        base_hash: &str,
    ) -> Result<SkillWriteOutcome, SkillHomeError> {
        validate_skill_slug(slug)?;
        let current = self.read(slug)?;
        if current.summary.content_hash != base_hash {
            return Err(SkillHomeError::HashConflict);
        }
        if !current.summary.enabled {
            return Ok(SkillWriteOutcome {
                document: current,
                changed: false,
            });
        }
        let mut parsed = parse_skill_markdown(&current.markdown)?;
        parsed.enabled = false;
        let normalized = render_parsed_skill(parsed, false)?;
        self.update(slug, &normalized, base_hash)
    }

    /// Hard-delete only a Home package. A same-slug builtin becomes visible again.
    pub fn hard_delete(&self, slug: &str, base_hash: &str) -> Result<(), SkillHomeError> {
        validate_skill_slug(slug)?;
        let _guard = self.inner.write_lock.lock();
        let current = self.read(slug)?;
        if current.summary.source != SkillSource::Home {
            return Err(SkillHomeError::NotFound(slug.to_string()));
        }
        if current.summary.content_hash != base_hash {
            return Err(SkillHomeError::HashConflict);
        }
        let directory = self.inner.root.join(slug);
        fs::remove_dir_all(&directory).map_err(|source| io_error(&directory, source))?;
        self.mark_pending_stale(slug)?;
        bump_machine_epoch();
        Ok(())
    }

    /// Install a new package. Existing Home or builtin slugs cannot be overwritten.
    pub fn install_new(
        &self,
        slug: &str,
        markdown: &str,
        files: &[(PathBuf, Vec<u8>)],
    ) -> Result<SkillDocument, SkillHomeError> {
        validate_skill_slug(slug)?;
        let _guard = self.inner.write_lock.lock();
        if self.resolve_path(slug).is_ok() {
            return Err(SkillHomeError::AlreadyExists(slug.to_string()));
        }
        reject_secrets(markdown)?;
        let normalized = normalize_skill_markdown(markdown, None)?;
        let directory = self.inner.root.join(slug);
        fs::create_dir_all(&directory).map_err(|source| io_error(&directory, source))?;
        for (relative, bytes) in files {
            validate_package_relative_path(relative)?;
            if relative == Path::new(SKILL_FILE)
                || relative
                    .components()
                    .any(|part| part.as_os_str() == HISTORY_DIR)
            {
                return Err(SkillHomeError::InvalidPackagePath(
                    relative.display().to_string(),
                ));
            }
            if let Ok(text) = std::str::from_utf8(bytes) {
                reject_secrets(text)?;
            }
            let target = directory.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
            }
            atomic_write(&target, bytes)?;
        }
        if let Err(error) = self.write_head_and_history(slug, &normalized) {
            let _ = fs::remove_dir_all(&directory);
            return Err(error);
        }
        self.mark_pending_stale(slug)?;
        bump_machine_epoch();
        self.read(slug)
    }

    pub fn history(&self, slug: &str) -> Result<Vec<SkillHistoryEntry>, SkillHomeError> {
        validate_skill_slug(slug)?;
        let directory = self.inner.root.join(slug).join(HISTORY_DIR);
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut entries = Vec::new();
        for entry in fs::read_dir(&directory).map_err(|source| io_error(&directory, source))? {
            let entry = entry.map_err(|source| io_error(&directory, source))?;
            let path = entry.path();
            let Some(revision) = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| value.parse::<u64>().ok())
            else {
                continue;
            };
            let markdown = read_string(&path)?;
            let updated_at = fs::metadata(&path)
                .and_then(|metadata| metadata.modified())
                .map(DateTime::<Utc>::from)
                .unwrap_or_else(|_| Utc::now());
            entries.push(SkillHistoryEntry {
                revision,
                content_hash: content_hash(&markdown),
                updated_at,
            });
        }
        entries.sort_by_key(|entry| entry.revision);
        Ok(entries)
    }

    pub fn history_document(
        &self,
        slug: &str,
        revision: u64,
    ) -> Result<SkillHistoryDocument, SkillHomeError> {
        validate_skill_slug(slug)?;
        let path = self
            .inner
            .root
            .join(slug)
            .join(HISTORY_DIR)
            .join(format!("{revision}.md"));
        if !path.is_file() {
            return Err(SkillHomeError::NotFound(format!("{slug}@{revision}")));
        }
        let markdown = read_string(&path)?;
        Ok(SkillHistoryDocument {
            slug: slug.to_string(),
            revision,
            content_hash: content_hash(&markdown),
            markdown,
        })
    }

    pub fn create_request(
        &self,
        input: CreateSkillProposal,
    ) -> Result<SkillProposal, SkillHomeError> {
        validate_skill_slug(&input.slug)?;
        let _guard = self.inner.write_lock.lock();
        validate_proposal_input(&input)?;
        normalize_skill_markdown(
            &input.proposed_markdown,
            input.source.is_automatic().then_some(true),
        )?;
        if self.list_requests_unlocked()?.iter().any(|request| {
            request.slug == input.slug && request.status == SkillProposalStatus::Pending
        }) {
            return Err(SkillHomeError::RequestExists(input.slug));
        }
        let now = Utc::now();
        let request = SkillProposal {
            id: Uuid::new_v4().to_string(),
            slug: input.slug,
            title: input.title.trim().to_string(),
            proposed_markdown: input.proposed_markdown,
            evidence: input.evidence,
            attestation: input.attestation.map(|value| value.trim().to_string()),
            base_hash: input.base_hash,
            source: input.source,
            reason: input.reason.trim().to_string(),
            status: SkillProposalStatus::Pending,
            created_at: now,
            updated_at: now,
        };
        self.write_request(&request)?;
        Ok(request)
    }

    pub fn list_requests(&self) -> Result<Vec<SkillProposal>, SkillHomeError> {
        let _guard = self.inner.write_lock.lock();
        self.list_requests_unlocked()
    }

    pub fn get_request(&self, id: &str) -> Result<SkillProposal, SkillHomeError> {
        validate_request_id(id)?;
        let path = self.request_path(id);
        if !path.is_file() {
            return Err(SkillHomeError::NotFound(id.to_string()));
        }
        read_json(&path)
    }

    pub fn reject_request(&self, id: &str) -> Result<SkillProposal, SkillHomeError> {
        let _guard = self.inner.write_lock.lock();
        let mut request = self.get_request(id)?;
        if request.status != SkillProposalStatus::Pending {
            return Err(SkillHomeError::RequestStale);
        }
        request.status = SkillProposalStatus::Rejected;
        request.updated_at = Utc::now();
        self.write_request(&request)?;
        Ok(request)
    }

    pub fn accept_request(&self, id: &str) -> Result<SkillProposal, SkillHomeError> {
        let _guard = self.inner.write_lock.lock();
        let mut request = self.get_request(id)?;
        if request.status != SkillProposalStatus::Pending {
            return Err(SkillHomeError::RequestStale);
        }
        validate_persisted_proposal(&request)?;
        let current_hash = self
            .read(&request.slug)
            .map(|document| document.summary.content_hash)
            .or_else(|error| match error {
                SkillHomeError::NotFound(_) => Ok(ZERO_HASH.to_string()),
                other => Err(other),
            })?;
        if current_hash != request.base_hash {
            request.status = SkillProposalStatus::Stale;
            request.updated_at = Utc::now();
            self.write_request(&request)?;
            return Err(SkillHomeError::RequestStale);
        }
        reject_secrets(&request.proposed_markdown)?;
        let normalized = normalize_skill_markdown(
            &request.proposed_markdown,
            request.source.is_automatic().then_some(true),
        )?;
        self.write_head_and_history(&request.slug, &normalized)?;
        request.status = SkillProposalStatus::Accepted;
        request.updated_at = Utc::now();
        self.write_request(&request)?;
        bump_machine_epoch();
        Ok(request)
    }

    /// Repair a head written before its request status was durably accepted.
    pub fn reconcile_pending_heads(&self) -> Result<usize, SkillHomeError> {
        let _guard = self.inner.write_lock.lock();
        let mut repaired = 0;
        for mut request in self.list_requests_unlocked()? {
            if request.status != SkillProposalStatus::Pending {
                continue;
            }
            let normalized = normalize_skill_markdown(
                &request.proposed_markdown,
                request.source.is_automatic().then_some(true),
            )?;
            let Ok(document) = self.read(&request.slug) else {
                continue;
            };
            if document.summary.content_hash != content_hash(&normalized) {
                continue;
            }
            self.ensure_history_snapshot(&request.slug, &normalized)?;
            request.status = SkillProposalStatus::Accepted;
            request.updated_at = Utc::now();
            self.write_request(&request)?;
            repaired += 1;
        }
        if repaired > 0 {
            bump_machine_epoch();
        }
        Ok(repaired)
    }

    fn collect_slugs(
        &self,
        root: &Path,
        source: SkillSource,
        slugs: &mut BTreeMap<String, SkillSource>,
    ) -> Result<(), SkillHomeError> {
        if !root.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(root).map_err(|source| io_error(root, source))? {
            let entry = entry.map_err(|source| io_error(root, source))?;
            if !entry.path().is_dir() {
                continue;
            }
            let slug = entry.file_name().to_string_lossy().to_string();
            if slug == REQUESTS_DIR || validate_skill_slug(&slug).is_err() {
                continue;
            }
            if entry.path().join(SKILL_FILE).is_file() {
                slugs.insert(slug, source.clone());
            }
        }
        Ok(())
    }

    fn resolve_path(&self, slug: &str) -> Result<(PathBuf, SkillSource), SkillHomeError> {
        let home = self.inner.root.join(slug).join(SKILL_FILE);
        if home.is_file() {
            return Ok((home, SkillSource::Home));
        }
        let builtin = self.inner.builtin_root.join(slug).join(SKILL_FILE);
        if builtin.is_file() {
            return Ok((builtin, SkillSource::Builtin));
        }
        Err(SkillHomeError::NotFound(slug.to_string()))
    }

    fn write_head_and_history(&self, slug: &str, markdown: &str) -> Result<(), SkillHomeError> {
        let path = self.inner.root.join(slug).join(SKILL_FILE);
        atomic_write(&path, markdown.as_bytes())?;
        self.ensure_history_snapshot(slug, markdown)
    }

    fn ensure_history_snapshot(&self, slug: &str, markdown: &str) -> Result<(), SkillHomeError> {
        let entries = self.history(slug)?;
        if entries
            .last()
            .is_some_and(|entry| entry.content_hash == content_hash(markdown))
        {
            return Ok(());
        }
        let revision = entries.last().map_or(1, |entry| entry.revision + 1);
        let path = self
            .inner
            .root
            .join(slug)
            .join(HISTORY_DIR)
            .join(format!("{revision}.md"));
        atomic_write(&path, markdown.as_bytes())
    }

    fn request_path(&self, id: &str) -> PathBuf {
        self.inner
            .root
            .join(REQUESTS_DIR)
            .join(format!("{id}.json"))
    }

    fn write_request(&self, request: &SkillProposal) -> Result<(), SkillHomeError> {
        let bytes = serde_json::to_vec_pretty(request)
            .map_err(|error| SkillHomeError::Serialization(error.to_string()))?;
        atomic_write(&self.request_path(&request.id), &bytes)
    }

    fn list_requests_unlocked(&self) -> Result<Vec<SkillProposal>, SkillHomeError> {
        let directory = self.inner.root.join(REQUESTS_DIR);
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut requests: Vec<SkillProposal> = Vec::new();
        for entry in fs::read_dir(&directory).map_err(|source| io_error(&directory, source))? {
            let entry = entry.map_err(|source| io_error(&directory, source))?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            requests.push(read_json(&entry.path())?);
        }
        requests.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(requests)
    }

    fn mark_pending_stale(&self, slug: &str) -> Result<(), SkillHomeError> {
        for mut request in self.list_requests_unlocked()? {
            if request.slug == slug && request.status == SkillProposalStatus::Pending {
                request.status = SkillProposalStatus::Stale;
                request.updated_at = Utc::now();
                self.write_request(&request)?;
            }
        }
        Ok(())
    }
}

pub fn validate_skill_slug(slug: &str) -> Result<(), SkillHomeError> {
    let pattern = Regex::new(r"^[a-z0-9][a-z0-9-]{0,62}$")
        .map_err(|error| SkillHomeError::Serialization(error.to_string()))?;
    let reserved: HashSet<&'static str> = [
        "requests", "history", "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5",
        "com6", "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7",
        "lpt8", "lpt9",
    ]
    .into_iter()
    .collect();
    if !pattern.is_match(slug) || reserved.contains(slug) {
        return Err(SkillHomeError::SlugInvalid(slug.to_string()));
    }
    Ok(())
}

fn validate_request_id(id: &str) -> Result<(), SkillHomeError> {
    Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| SkillHomeError::NotFound(id.to_string()))
}

pub fn content_hash(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

/// Parse and normalize frontmatter while removing legacy runtime `always` keys.
pub fn normalize_skill_markdown(
    markdown: &str,
    force_always_false: Option<bool>,
) -> Result<String, SkillHomeError> {
    let parsed = parse_skill_markdown(markdown)?;
    render_parsed_skill(parsed, force_always_false == Some(true))
}

fn render_parsed_skill(
    mut parsed: ParsedSkill,
    force_always_false: bool,
) -> Result<String, SkillHomeError> {
    let enabled_key = Value::String("enabled".into());
    let always_key = Value::String("always".into());
    parsed.yaml.insert(enabled_key, Value::Bool(parsed.enabled));
    parsed.yaml.insert(
        always_key,
        Value::Bool(if force_always_false {
            false
        } else {
            parsed.always
        }),
    );
    strip_legacy_always(&mut parsed.yaml);
    let yaml = serde_yaml::to_string(&parsed.yaml)
        .map_err(|error| SkillHomeError::Serialization(error.to_string()))?;
    Ok(format!("---\n{}---\n{}", yaml, parsed.body))
}

fn parse_skill_markdown(markdown: &str) -> Result<ParsedSkill, SkillHomeError> {
    let normalized = markdown.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return Err(SkillHomeError::InvalidMarkdown(
            "YAML frontmatter opening delimiter is required".into(),
        ));
    };
    let Some((yaml_text, body)) = rest.split_once("\n---\n") else {
        return Err(SkillHomeError::InvalidMarkdown(
            "YAML frontmatter closing delimiter is required".into(),
        ));
    };
    let yaml: Mapping = serde_yaml::from_str(yaml_text)
        .map_err(|error| SkillHomeError::InvalidMarkdown(error.to_string()))?;
    let description = yaml
        .get(Value::String("description".into()))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            SkillHomeError::InvalidMarkdown("description must be a non-empty string".into())
        })?
        .to_string();
    let enabled = bool_field(&yaml, "enabled", true)?;
    let always = bool_field(&yaml, "always", false)?;
    Ok(ParsedSkill {
        description,
        enabled,
        always,
        body: body.to_string(),
        yaml,
    })
}

fn bool_field(yaml: &Mapping, key: &str, default: bool) -> Result<bool, SkillHomeError> {
    match yaml.get(Value::String(key.into())) {
        None => Ok(default),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(SkillHomeError::InvalidMarkdown(format!(
            "{key} must be a boolean"
        ))),
    }
}

fn strip_legacy_always(yaml: &mut Mapping) {
    let Some(Value::Mapping(metadata)) = yaml.get_mut(Value::String("metadata".into())) else {
        return;
    };
    for runtime in ["nanobot", "openclaw"] {
        if let Some(Value::Mapping(values)) = metadata.get_mut(Value::String(runtime.into())) {
            values.remove(Value::String("always".into()));
        }
    }
}

fn validate_proposal_input(input: &CreateSkillProposal) -> Result<(), SkillHomeError> {
    if input.title.trim().is_empty() || input.reason.trim().is_empty() {
        return Err(SkillHomeError::InvalidMarkdown(
            "proposal title and reason are required".into(),
        ));
    }
    let has_evidence =
        !input.evidence.is_empty() && input.evidence.iter().all(SkillEvidence::is_valid);
    let has_attestation = input
        .attestation
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    if input.source.is_automatic() && !has_evidence
        || input.source == SkillProposalSource::UserRequest && !has_evidence && !has_attestation
    {
        return Err(SkillHomeError::EvidenceRequired);
    }
    if !input.evidence.is_empty() && !has_evidence {
        return Err(SkillHomeError::EvidenceRequired);
    }
    let current_is_zero = input.base_hash == ZERO_HASH;
    if !current_is_zero
        && (input.base_hash.len() != 64 || !input.base_hash.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err(SkillHomeError::InvalidMarkdown(
            "base_hash must be 0 or SHA-256".into(),
        ));
    }
    Ok(())
}

fn validate_persisted_proposal(request: &SkillProposal) -> Result<(), SkillHomeError> {
    validate_proposal_input(&CreateSkillProposal {
        slug: request.slug.clone(),
        title: request.title.clone(),
        proposed_markdown: request.proposed_markdown.clone(),
        evidence: request.evidence.clone(),
        attestation: request.attestation.clone(),
        base_hash: request.base_hash.clone(),
        source: request.source.clone(),
        reason: request.reason.clone(),
    })
}

fn reject_secrets(content: &str) -> Result<(), SkillHomeError> {
    let patterns = [
        r"(?i)-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----",
        r"(?i)\b(?:sk|pk)-[a-z0-9_-]{20,}\b",
        r"\bgh[pousr]_[A-Za-z0-9]{30,}\b",
        r#"(?i)\b(?:api[_-]?key|access[_-]?token|client[_-]?secret|password)\s*[:=]\s*['"]?[A-Za-z0-9_./+=-]{16,}"#,
    ];
    for pattern in patterns {
        let regex = Regex::new(pattern)
            .map_err(|error| SkillHomeError::Serialization(error.to_string()))?;
        if regex.is_match(content) {
            return Err(SkillHomeError::SecretRejected);
        }
    }
    Ok(())
}

fn validate_package_relative_path(path: &Path) -> Result<(), SkillHomeError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(SkillHomeError::InvalidPackagePath(
            path.display().to_string(),
        ));
    }
    Ok(())
}

fn bump_machine_epoch() {
    MACHINE_SKILL_EPOCH.fetch_add(1, Ordering::AcqRel);
}

fn read_string(path: &Path) -> Result<String, SkillHomeError> {
    fs::read_to_string(path).map_err(|source| io_error(path, source))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, SkillHomeError> {
    let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
    serde_json::from_slice(&bytes).map_err(|error| SkillHomeError::Serialization(error.to_string()))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), SkillHomeError> {
    let parent = path
        .parent()
        .ok_or_else(|| SkillHomeError::InvalidPackagePath(path.display().to_string()))?;
    fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill-write");
    let temp = path.with_file_name(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        nanos + u128::from(counter)
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(|source| io_error(&temp, source))?;
        file.write_all(bytes)
            .map_err(|source| io_error(&temp, source))?;
        file.sync_all().map_err(|source| io_error(&temp, source))?;
        fs::rename(&temp, path).map_err(|source| io_error(path, source))?;
        if let Ok(directory) = File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn io_error(path: &Path, source: std::io::Error) -> SkillHomeError {
    SkillHomeError::Io {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn markdown(
        description: &str,
        enabled: Option<bool>,
        always: Option<bool>,
        body: &str,
    ) -> String {
        let enabled = enabled
            .map(|value| format!("enabled: {value}\n"))
            .unwrap_or_default();
        let always = always
            .map(|value| format!("always: {value}\n"))
            .unwrap_or_default();
        format!("---\nname: demo\ndescription: {description}\n{enabled}{always}---\n{body}")
    }

    fn home() -> (TempDir, TempDir, SkillHome) {
        let config = TempDir::new().unwrap();
        let builtin = TempDir::new().unwrap();
        let home = SkillHome::new(config.path(), builtin.path());
        (config, builtin, home)
    }

    fn write_builtin(root: &Path, slug: &str, content: &str) {
        fs::create_dir_all(root.join(slug)).unwrap();
        fs::write(root.join(slug).join(SKILL_FILE), content).unwrap();
    }

    #[test]
    fn root_is_lazy_and_workspace_is_never_imported() {
        let (config, builtin, home) = home();
        fs::create_dir_all(config.path().join("workspace/skills/legacy")).unwrap();
        fs::write(
            config.path().join("workspace/skills/legacy/SKILL.md"),
            markdown("Legacy", None, None, "legacy"),
        )
        .unwrap();
        assert!(!home.root().exists());
        assert!(home.list().unwrap().is_empty());
        assert!(builtin.path().exists());
        assert!(!home.root().exists());
    }

    #[test]
    fn home_override_disable_and_delete_restore_builtin() {
        let (_config, builtin, home) = home();
        write_builtin(
            builtin.path(),
            "demo",
            &markdown("Builtin", None, None, "builtin"),
        );
        let builtin_doc = home.read("demo").unwrap();
        let updated = home
            .update(
                "demo",
                &markdown("Home", Some(true), Some(false), "home"),
                &builtin_doc.summary.content_hash,
            )
            .unwrap();
        assert_eq!(updated.document.summary.source, SkillSource::Home);
        let disabled = home
            .disable("demo", &updated.document.summary.content_hash)
            .unwrap();
        assert!(!disabled.document.summary.enabled);
        assert!(matches!(
            home.read_enabled("demo"),
            Err(SkillHomeError::NotFound(_))
        ));
        home.hard_delete("demo", &disabled.document.summary.content_hash)
            .unwrap();
        assert_eq!(home.read("demo").unwrap().summary.description, "Builtin");
    }

    #[test]
    fn slug_frontmatter_and_legacy_always_are_strict() {
        for slug in [
            "requests",
            "history",
            "con",
            "COM1",
            "-bad",
            "bad_underscore",
        ] {
            assert!(validate_skill_slug(slug).is_err(), "{slug}");
        }
        assert!(validate_skill_slug("a-good-1").is_ok());
        assert!(parse_skill_markdown("no-frontmatter").is_err());
        assert!(parse_skill_markdown("---\ndescription: x\nenabled: yes\n---\nbody").is_err());
        let raw = "---\ndescription: Demo\nmetadata:\n  nanobot:\n    always: true\n    emoji: wave\n  openclaw:\n    always: true\nalways: true\n---\nbody";
        let normalized = normalize_skill_markdown(raw, None).unwrap();
        let yaml = normalized.split("---\n").nth(1).unwrap();
        assert!(!yaml.contains("always: true\n    emoji"));
        assert!(yaml.contains("emoji: wave"));
        assert_eq!(normalized.matches("always: true").count(), 1);
    }

    #[test]
    fn cas_noop_history_secret_and_stale_request() {
        let (_config, _builtin, home) = home();
        let initial = markdown("Demo", None, None, "initial");
        let installed = home.install_new("demo", &initial, &[]).unwrap();
        assert_eq!(home.history("demo").unwrap().len(), 1);
        let normalized = normalize_skill_markdown(&initial, None).unwrap();
        let noop = home
            .update("demo", &normalized, &installed.summary.content_hash)
            .unwrap();
        assert!(!noop.changed);
        assert_eq!(home.history("demo").unwrap().len(), 1);
        let request = home
            .create_request(CreateSkillProposal {
                slug: "demo".into(),
                title: "Update".into(),
                proposed_markdown: markdown("Next", None, None, "next"),
                evidence: vec![SkillEvidence {
                    session_key: Some("s1".into()),
                    actmem_pointer: None,
                    autodream_run_id: None,
                    tool: None,
                    artifact: None,
                }],
                attestation: None,
                base_hash: noop.document.summary.content_hash.clone(),
                source: SkillProposalSource::Distill,
                reason: "Reusable".into(),
            })
            .unwrap();
        let changed = home
            .update(
                "demo",
                &markdown("Direct", None, None, "direct"),
                &noop.document.summary.content_hash,
            )
            .unwrap();
        assert!(changed.changed);
        assert_eq!(
            home.get_request(&request.id).unwrap().status,
            SkillProposalStatus::Stale
        );
        assert_eq!(home.history("demo").unwrap().len(), 2);
        assert!(matches!(
            home.update(
                "demo",
                &markdown("Secret", None, None, "api_key=abcdefghijklmnop1234"),
                &changed.document.summary.content_hash
            ),
            Err(SkillHomeError::SecretRejected)
        ));
    }

    #[test]
    fn proposal_evidence_attestation_accept_reject_and_orphan_repair() {
        let (_config, _builtin, home) = home();
        let auto = CreateSkillProposal {
            slug: "auto".into(),
            title: "Auto".into(),
            proposed_markdown: markdown("Auto", None, Some(true), "automatic"),
            evidence: Vec::new(),
            attestation: None,
            base_hash: ZERO_HASH.into(),
            source: SkillProposalSource::Autodream,
            reason: "Pattern".into(),
        };
        assert!(matches!(
            home.create_request(auto.clone()),
            Err(SkillHomeError::EvidenceRequired)
        ));
        let mut user = auto.clone();
        user.slug = "user".into();
        user.source = SkillProposalSource::UserRequest;
        user.attestation = Some("I confirm this skill".into());
        let user = home.create_request(user).unwrap();
        assert!(matches!(
            home.create_request(CreateSkillProposal {
                slug: "user".into(),
                title: "Duplicate".into(),
                proposed_markdown: markdown("User", None, None, "duplicate"),
                evidence: Vec::new(),
                attestation: Some("confirm".into()),
                base_hash: ZERO_HASH.into(),
                source: SkillProposalSource::UserRequest,
                reason: "Duplicate".into(),
            }),
            Err(SkillHomeError::RequestExists(_))
        ));
        let accepted = home.accept_request(&user.id).unwrap();
        assert_eq!(accepted.status, SkillProposalStatus::Accepted);
        assert!(home.read("user").unwrap().summary.can_hard_delete);

        let mut auto = auto;
        auto.evidence.push(SkillEvidence {
            session_key: None,
            actmem_pointer: None,
            autodream_run_id: Some("run-1".into()),
            tool: None,
            artifact: None,
        });
        let auto = home.create_request(auto).unwrap();
        let normalized = normalize_skill_markdown(&auto.proposed_markdown, Some(true)).unwrap();
        home.write_head_and_history("auto", &normalized).unwrap();
        fs::remove_file(home.root().join("auto/history/1.md")).unwrap();
        assert_eq!(home.reconcile_pending_heads().unwrap(), 1);
        assert_eq!(
            home.get_request(&auto.id).unwrap().status,
            SkillProposalStatus::Accepted
        );
        assert!(!home.read("auto").unwrap().summary.always);
        assert_eq!(home.history("auto").unwrap().len(), 1);
    }

    #[test]
    fn install_rejects_existing_and_service_history_paths() {
        let (_config, builtin, home) = home();
        write_builtin(
            builtin.path(),
            "demo",
            &markdown("Demo", None, None, "builtin"),
        );
        assert!(matches!(
            home.install_new("demo", &markdown("Other", None, None, "other"), &[]),
            Err(SkillHomeError::AlreadyExists(_))
        ));
        assert!(matches!(
            home.install_new(
                "new",
                &markdown("New", None, None, "new"),
                &[(PathBuf::from("history/99.md"), b"fake".to_vec())]
            ),
            Err(SkillHomeError::InvalidPackagePath(_))
        ));
    }
}
