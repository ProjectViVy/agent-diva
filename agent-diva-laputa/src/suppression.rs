use std::{fs, time::Duration};

use agent_diva_core::evolution::{memory_candidate_content_digest, EvolutionProposal};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{atomic_write_json, LaputaLock, LaputaStorage, LockOptions, Result};

const SCHEMA_VERSION: u32 = 1;
const MAX_SUPPRESSIONS: usize = 1_000;
const DEFAULT_TTL_DAYS: i64 = 90;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateSuppression {
    pub content_digest: String,
    pub proposal_id: String,
    pub rejected_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CandidateSuppressionFile {
    schema_version: u32,
    entries: Vec<CandidateSuppression>,
}

impl Default for CandidateSuppressionFile {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CandidateSuppressionStore {
    storage: LaputaStorage,
}

impl CandidateSuppressionStore {
    pub fn new(storage: LaputaStorage) -> Self {
        Self { storage }
    }

    pub fn record_rejection(
        &self,
        proposal: &EvolutionProposal,
        rejected_at: DateTime<Utc>,
    ) -> Result<CandidateSuppression> {
        let _lock = LaputaLock::acquire(
            self.storage.paths().lock_file("candidate-suppression"),
            LockOptions {
                timeout: Duration::from_secs(5),
                ..LockOptions::default()
            },
        )?;
        let mut file = self.read_file()?;
        file.entries.retain(|entry| entry.expires_at > rejected_at);
        let suppression = CandidateSuppression {
            content_digest: memory_candidate_content_digest(&proposal.proposed_patch),
            proposal_id: proposal.id.clone(),
            rejected_at,
            expires_at: rejected_at + chrono::Duration::days(DEFAULT_TTL_DAYS),
        };
        if let Some(existing) = file
            .entries
            .iter_mut()
            .find(|entry| entry.content_digest == suppression.content_digest)
        {
            *existing = suppression.clone();
        } else {
            file.entries.push(suppression.clone());
        }
        file.entries
            .sort_by(|left, right| left.rejected_at.cmp(&right.rejected_at));
        trim_to_capacity(&mut file.entries);
        atomic_write_json(self.storage.paths().suppression_json(), &file)?;
        Ok(suppression)
    }

    pub fn active_digests(&self, now: DateTime<Utc>) -> Result<Vec<String>> {
        Ok(self
            .read_file()?
            .entries
            .into_iter()
            .filter(|entry| entry.expires_at > now)
            .map(|entry| entry.content_digest)
            .collect())
    }

    pub fn is_suppressed(&self, content: &str, now: DateTime<Utc>) -> Result<bool> {
        let digest = memory_candidate_content_digest(content);
        Ok(self
            .active_digests(now)?
            .iter()
            .any(|entry| entry == &digest))
    }

    fn read_file(&self) -> Result<CandidateSuppressionFile> {
        let path = self.storage.paths().suppression_json();
        if !path.exists() {
            return Ok(CandidateSuppressionFile::default());
        }
        let bytes = fs::read(&path).map_err(|source| crate::LaputaError::io(&path, source))?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

fn trim_to_capacity(entries: &mut Vec<CandidateSuppression>) {
    if entries.len() > MAX_SUPPRESSIONS {
        entries.drain(..entries.len() - MAX_SUPPRESSIONS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_suppression_capacity_keeps_newest_entries() {
        let now = Utc::now();
        let mut entries = (0..=MAX_SUPPRESSIONS)
            .map(|index| CandidateSuppression {
                content_digest: format!("sha256:{index}"),
                proposal_id: format!("proposal-{index}"),
                rejected_at: now + chrono::Duration::seconds(index as i64),
                expires_at: now + chrono::Duration::days(1),
            })
            .collect::<Vec<_>>();

        trim_to_capacity(&mut entries);

        assert_eq!(entries.len(), MAX_SUPPRESSIONS);
        assert_eq!(entries[0].proposal_id, "proposal-1");
    }
}
