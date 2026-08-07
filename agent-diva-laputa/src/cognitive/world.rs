//! WORLD.MD — the claim-based actionable world model.
//!
//! Schema is aligned with garden ADR-0004 §3.2: each claim is a
//! `## [domain] title` heading followed by status/confidence/scope/source/
//! updated metadata and a body of at most [`MAX_CLAIM_TEXT_CHARS`]
//! characters.
//!
//! Write boundaries:
//! - Humans may edit the file directly (unrestricted).
//! - Governance writes go through [`WorldStore::governed_upsert`] and may
//!   never overwrite a `confirmed + source=user` claim; the only allowed
//!   change is marking it stale with an appended note.
//! - The read path only ever returns bounded, scope-matched projections
//!   ([`WorldStore::project`]); the file is never injected wholesale.

use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};

use crate::{atomic_write, LaputaError, Result};

/// Default projection budget in characters (garden ADR-0004 §3.3).
pub const DEFAULT_PROJECTION_BUDGET: usize = 4000;
/// Hard upper bound for any projection budget.
pub const MAX_PROJECTION_BUDGET: usize = 16000;
/// Maximum body length of a single claim.
pub const MAX_CLAIM_TEXT_CHARS: usize = 280;
/// Actor id representing direct human edits.
pub const USER_ACTOR: &str = "user";

/// Claim lifecycle status (five states, garden ADR-0004 §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimStatus {
    Confirmed,
    Observed,
    Inferred,
    Hypothesis,
    Stale,
}

impl ClaimStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Observed => "observed",
            Self::Inferred => "inferred",
            Self::Hypothesis => "hypothesis",
            Self::Stale => "stale",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "confirmed" => Some(Self::Confirmed),
            "observed" => Some(Self::Observed),
            "inferred" => Some(Self::Inferred),
            "hypothesis" => Some(Self::Hypothesis),
            "stale" => Some(Self::Stale),
            _ => None,
        }
    }
}

/// A single world-model claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldClaim {
    pub domain: String,
    pub title: String,
    pub status: ClaimStatus,
    pub confidence: String,
    pub scopes: Vec<String>,
    pub source: String,
    pub updated: DateTime<Utc>,
    pub text: String,
}

impl WorldClaim {
    /// A claim is user-protected when a human confirmed it; governance
    /// writes may only mark it stale, never replace it.
    pub fn is_user_confirmed(&self) -> bool {
        self.status == ClaimStatus::Confirmed && self.source == USER_ACTOR
    }

    fn matches_scopes(&self, scopes: &[String]) -> bool {
        scopes
            .iter()
            .any(|wanted| self.scopes.iter().any(|scope| scope == wanted))
    }

    fn rendered_cost(&self) -> usize {
        self.text.chars().count()
    }
}

/// Errors specific to WORLD governance writes.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WorldError {
    #[error("cannot overwrite user-confirmed claim [{domain}] {title}")]
    ConfirmedProtected { domain: String, title: String },
    #[error("claim body exceeds {max} characters")]
    BodyTooLong { max: usize },
    #[error("claim heading must be non-empty")]
    EmptyHeading,
}

/// File-backed store for WORLD.MD claims.
#[derive(Debug, Clone)]
pub struct WorldStore {
    path: PathBuf,
    claims: Vec<WorldClaim>,
}

impl WorldStore {
    /// Load claims from disk; a missing file yields an empty world.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let claims = match fs::read_to_string(&path) {
            Ok(raw) => parse_world(&raw),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(source) => return Err(LaputaError::io(&path, source)),
        };
        Ok(Self { path, claims })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn claims(&self) -> &[WorldClaim] {
        &self.claims
    }

    pub fn total(&self) -> usize {
        self.claims.len()
    }

    /// Bounded, scope-matched projection. Budget is clamped into
    /// [`DEFAULT_PROJECTION_BUDGET`]..=[`MAX_PROJECTION_BUDGET`]. Without
    /// scopes, claims are ordered by confidence (high first) for stability.
    pub fn project(&self, scopes: &[String], budget_chars: usize) -> Vec<WorldClaim> {
        let budget = budget_chars.clamp(DEFAULT_PROJECTION_BUDGET, MAX_PROJECTION_BUDGET);

        let mut snapshot = self.claims.clone();
        if scopes.is_empty() {
            snapshot.sort_by_key(|claim| std::cmp::Reverse(confidence_rank(&claim.confidence)));
        }

        let mut result = Vec::new();
        let mut used = 0usize;
        for claim in snapshot {
            if !scopes.is_empty() && !claim.matches_scopes(scopes) {
                continue;
            }
            let cost = claim.rendered_cost();
            if used + cost > budget {
                break;
            }
            result.push(claim);
            used += cost;
        }
        result
    }

    /// Direct user save (unrestricted) or full-file rewrite of the current
    /// claim set. Governance writers must use [`Self::governed_upsert`].
    pub fn save(&self) -> Result<()> {
        atomic_write(&self.path, serialize_world(&self.claims).as_bytes())
    }

    /// Apply a governed claim upsert under actor semantics:
    /// - `actor == "user"`: unrestricted upsert;
    /// - otherwise: overwriting a user-confirmed claim is rejected with
    ///   [`WorldError::ConfirmedProtected`] unless the write only marks it
    ///   stale (protection note is appended to the existing body).
    pub fn governed_upsert(
        &mut self,
        actor: &str,
        claim: WorldClaim,
    ) -> std::result::Result<(), WorldError> {
        if claim.text.chars().count() > MAX_CLAIM_TEXT_CHARS {
            return Err(WorldError::BodyTooLong {
                max: MAX_CLAIM_TEXT_CHARS,
            });
        }
        if claim.domain.trim().is_empty() || claim.title.trim().is_empty() {
            return Err(WorldError::EmptyHeading);
        }

        if let Some(existing) = self
            .claims
            .iter_mut()
            .find(|item| item.domain == claim.domain && item.title == claim.title)
        {
            if existing.is_user_confirmed() && actor != USER_ACTOR {
                if claim.status != ClaimStatus::Stale {
                    return Err(WorldError::ConfirmedProtected {
                        domain: claim.domain,
                        title: claim.title,
                    });
                }
                // Marking stale is the only allowed change; keep the
                // confirmed body and append the governance note.
                existing.status = ClaimStatus::Stale;
                existing.source = actor.to_string();
                existing.updated = claim.updated;
                if !claim.text.trim().is_empty() {
                    existing.text = format!("{}\n[note: {}]", existing.text, claim.text.trim());
                }
                return Ok(());
            }
            *existing = claim;
            return Ok(());
        }

        self.claims.push(claim);
        Ok(())
    }
}

fn confidence_rank(confidence: &str) -> u8 {
    match confidence {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

fn serialize_world(claims: &[WorldClaim]) -> String {
    let mut out = String::from("# WORLD\n\n");
    for claim in claims {
        out.push_str(&format!("## [{}] {}\n", claim.domain, claim.title));
        out.push_str(&format!("- status: {}\n", claim.status.as_str()));
        out.push_str(&format!("- confidence: {}\n", claim.confidence));
        out.push_str(&format!("- scope: {}\n", claim.scopes.join(", ")));
        out.push_str(&format!("- source: {}\n", claim.source));
        out.push_str(&format!(
            "- updated: {}\n",
            claim
                .updated
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        ));
        out.push('\n');
        out.push_str(&claim.text);
        out.push_str("\n\n");
    }
    out
}

fn parse_world(raw: &str) -> Vec<WorldClaim> {
    let mut claims: Vec<WorldClaim> = Vec::new();
    let mut current: Option<WorldClaim> = None;

    for line in raw.lines() {
        if let Some(heading) = line.strip_prefix("## [") {
            if let Some(claim) = current.take() {
                claims.push(claim);
            }
            let (domain, title) = parse_claim_heading(heading);
            current = Some(WorldClaim {
                domain,
                title,
                status: ClaimStatus::Observed,
                confidence: String::new(),
                scopes: Vec::new(),
                source: String::new(),
                updated: DateTime::UNIX_EPOCH,
                text: String::new(),
            });
            continue;
        }

        let Some(claim) = current.as_mut() else {
            continue;
        };

        if let Some(field) = line.strip_prefix("- ") {
            parse_claim_meta(claim, field);
            continue;
        }

        if !line.trim().is_empty() {
            if !claim.text.is_empty() {
                claim.text.push('\n');
            }
            claim.text.push_str(line);
        }
    }
    if let Some(claim) = current.take() {
        claims.push(claim);
    }
    claims
}

fn parse_claim_heading(heading: &str) -> (String, String) {
    match heading.find(']') {
        Some(idx) => (
            heading[..idx].trim().to_string(),
            heading[idx + 1..].trim().to_string(),
        ),
        None => (String::new(), heading.trim().to_string()),
    }
}

fn parse_claim_meta(claim: &mut WorldClaim, field: &str) {
    let Some((key, value)) = field.split_once(':') else {
        return;
    };
    let key = key.trim();
    let value = value.trim();
    match key {
        "status" => {
            if let Some(status) = ClaimStatus::parse(value) {
                claim.status = status;
            }
        }
        "confidence" => claim.confidence = value.to_string(),
        "scope" => {
            claim.scopes = value
                .split(',')
                .map(str::trim)
                .filter(|scope| !scope.is_empty())
                .map(str::to_string)
                .collect();
        }
        "source" => claim.source = value.to_string(),
        "updated" => {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
                claim.updated = parsed.with_timezone(&Utc);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(domain: &str, title: &str, status: ClaimStatus, source: &str) -> WorldClaim {
        WorldClaim {
            domain: domain.to_string(),
            title: title.to_string(),
            status,
            confidence: "high".to_string(),
            scopes: vec!["dev".to_string()],
            source: source.to_string(),
            updated: DateTime::UNIX_EPOCH,
            text: "body".to_string(),
        }
    }

    #[test]
    fn roundtrip_preserves_claims() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("WORLD.MD");
        let mut store = WorldStore::load(&path).unwrap();
        assert_eq!(store.total(), 0);

        store
            .governed_upsert(
                USER_ACTOR,
                WorldClaim {
                    text: "Windows 11, 64GB RAM".to_string(),
                    scopes: vec!["dev".to_string(), "infra".to_string()],
                    ..claim(
                        "environment",
                        "Development machine",
                        ClaimStatus::Confirmed,
                        USER_ACTOR,
                    )
                },
            )
            .unwrap();
        store.save().unwrap();

        let reloaded = WorldStore::load(&path).unwrap();
        assert_eq!(reloaded.total(), 1);
        let parsed = &reloaded.claims()[0];
        assert_eq!(parsed.domain, "environment");
        assert_eq!(parsed.title, "Development machine");
        assert_eq!(parsed.status, ClaimStatus::Confirmed);
        assert_eq!(parsed.scopes, ["dev", "infra"]);
        assert_eq!(parsed.source, "user");
        assert_eq!(parsed.text, "Windows 11, 64GB RAM");
    }

    #[test]
    fn project_respects_scope_and_budget() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = WorldStore::load(temp.path().join("WORLD.MD")).unwrap();
        // 20 medium-confidence dev claims of ~250 chars each: together they
        // exceed the default 4000-char budget.
        for i in 0..20 {
            store
                .governed_upsert(
                    USER_ACTOR,
                    WorldClaim {
                        title: format!("filler {i}"),
                        text: "b".repeat(250),
                        confidence: "medium".to_string(),
                        ..claim("env", "filler", ClaimStatus::Observed, USER_ACTOR)
                    },
                )
                .unwrap();
        }
        store
            .governed_upsert(
                USER_ACTOR,
                WorldClaim {
                    text: "small".to_string(),
                    confidence: "high".to_string(),
                    ..claim("env", "small dev claim", ClaimStatus::Observed, USER_ACTOR)
                },
            )
            .unwrap();
        store
            .governed_upsert(
                USER_ACTOR,
                WorldClaim {
                    scopes: vec!["finance".to_string()],
                    ..claim("env", "other scope", ClaimStatus::Observed, USER_ACTOR)
                },
            )
            .unwrap();

        // Default budget (4000) with an explicit scope keeps file order:
        // exactly 16 fillers of 250 chars fill the budget; the small claim
        // after them no longer fits.
        let projected = store.project(&["dev".to_string()], 0);
        assert_eq!(projected[0].title, "filler 0");
        assert_eq!(projected.len(), 16);

        // Without scopes, confidence ordering applies: the high-confidence
        // claim leads the projection.
        let unordered = store.project(&[], 0);
        assert_eq!(unordered[0].title, "small dev claim");

        // Max budget fits every dev claim.
        let loose = store.project(&["dev".to_string()], MAX_PROJECTION_BUDGET);
        assert_eq!(loose.len(), 21);

        // Scope filter keeps only matching claims.
        let finance = store.project(&["finance".to_string()], 0);
        assert_eq!(finance.len(), 1);
        assert_eq!(finance[0].title, "other scope");

        // Budgets above the hard cap are clamped back to 16000.
        let clamped = store.project(&["dev".to_string()], MAX_PROJECTION_BUDGET * 10);
        assert_eq!(clamped.len(), 21);
    }

    #[test]
    fn governed_upsert_protects_user_confirmed_claims() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = WorldStore::load(temp.path().join("WORLD.MD")).unwrap();
        store
            .governed_upsert(
                USER_ACTOR,
                claim("env", "dev machine", ClaimStatus::Confirmed, USER_ACTOR),
            )
            .unwrap();

        // Non-user overwrite is rejected.
        let error = store
            .governed_upsert(
                "autodream",
                WorldClaim {
                    status: ClaimStatus::Observed,
                    ..claim("env", "dev machine", ClaimStatus::Observed, "autodream")
                },
            )
            .unwrap_err();
        assert!(matches!(error, WorldError::ConfirmedProtected { .. }));

        // Non-user may only mark it stale with a note.
        store
            .governed_upsert(
                "autodream",
                WorldClaim {
                    status: ClaimStatus::Stale,
                    text: "hardware replaced".to_string(),
                    ..claim("env", "dev machine", ClaimStatus::Stale, "autodream")
                },
            )
            .unwrap();
        let updated = &store.claims()[0];
        assert_eq!(updated.status, ClaimStatus::Stale);
        assert_eq!(updated.source, "autodream");
        assert!(updated.text.starts_with("body"));
        assert!(updated.text.contains("[note: hardware replaced]"));

        // User may overwrite freely.
        store
            .governed_upsert(
                USER_ACTOR,
                claim("env", "dev machine", ClaimStatus::Confirmed, USER_ACTOR),
            )
            .unwrap();
        assert_eq!(store.claims()[0].status, ClaimStatus::Confirmed);
    }

    #[test]
    fn governed_upsert_validates_bounds() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = WorldStore::load(temp.path().join("WORLD.MD")).unwrap();

        let too_long = WorldClaim {
            text: "x".repeat(MAX_CLAIM_TEXT_CHARS + 1),
            ..claim("env", "long", ClaimStatus::Observed, "autodream")
        };
        assert!(matches!(
            store.governed_upsert("autodream", too_long),
            Err(WorldError::BodyTooLong { .. })
        ));

        let no_heading = WorldClaim {
            title: "   ".to_string(),
            ..claim("env", "blank", ClaimStatus::Observed, "autodream")
        };
        assert!(matches!(
            store.governed_upsert("autodream", no_heading),
            Err(WorldError::EmptyHeading)
        ));
    }
}
