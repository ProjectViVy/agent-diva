//! Governed Memory proposal decisions and non-production typed apply boundary.

use std::{path::Path, str::FromStr, sync::Arc};

use agent_diva_core::{
    evolution::{EvolutionProposal, RiskLevel},
    governance::{
        evaluate_policy, ApprovalGrant, ApprovalLedgerError, ApprovalReceipt, ApprovalRecord,
        ApprovalRequest, ApprovalState, ApprovalStatus, AuditCorrelation, AutonomyLevel,
        Capability, ContentDigest, Decision, DigestAlgorithm, GovernanceLedger, GovernanceSubject,
        GovernanceSubjectKind, PolicyContext, PolicyEvaluation, ResourceKind, ResourceScope,
        RiskClass, SqliteGovernanceLedger,
    },
    memory::memory_content_digest,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, SqlitePool,
};

use crate::{LaputaPaths, TypedMemoryStoreError};

const POLICY_VERSION: &str = "memory-apply-v1";
const REQUEST_TTL_HOURS: i64 = 24;

#[derive(Debug, thiserror::Error)]
pub enum MemoryGovernanceError {
    #[error("governance ledger failed: {0}")]
    Ledger(#[from] ApprovalLedgerError),
    #[error("governance persistence failed: {0}")]
    Persistence(#[from] sqlx::Error),
    #[error("proposal digest no longer matches its governance request")]
    StaleProposal,
    #[error("proposal has no allowed approval receipt")]
    ApprovalRequired,
    #[error("approval receipt grant is invalid for proposal risk")]
    InvalidGrant,
    #[error("typed apply failed: {0}")]
    TypedStore(#[from] TypedMemoryStoreError),
    #[error("governance filesystem initialization failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryGovernanceView {
    pub proposal_id: String,
    pub request_id: String,
    pub request_version: u64,
    pub status: ApprovalStatus,
    pub policy: PolicyEvaluation,
    pub receipt: Option<ApprovalReceipt>,
}

/// Caller-supplied fields for one governed proposal decision.
pub struct MemoryGovernanceDecision<'a> {
    pub decision: Decision,
    pub grant: ApprovalGrant,
    pub actor: GovernanceSubject,
    pub idempotency_key: &'a str,
    pub decided_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct MemoryGovernanceCoordinator {
    workspace_id: String,
    pool: SqlitePool,
    ledger: Arc<tokio::sync::OnceCell<SqliteGovernanceLedger>>,
}

impl std::fmt::Debug for MemoryGovernanceCoordinator {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MemoryGovernanceCoordinator")
            .field("workspace_id", &self.workspace_id)
            .finish_non_exhaustive()
    }
}

impl MemoryGovernanceCoordinator {
    pub fn open_lazy(
        workspace_root: impl AsRef<Path>,
        workspace_id: impl Into<String>,
    ) -> Result<Self, MemoryGovernanceError> {
        let paths = LaputaPaths::new(workspace_root.as_ref());
        std::fs::create_dir_all(paths.laputa_dir())?;
        let path = paths.governance_database();
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_lazy_with(options);
        Ok(Self {
            workspace_id: workspace_id.into(),
            pool,
            ledger: Arc::new(tokio::sync::OnceCell::new()),
        })
    }

    async fn ledger(&self) -> Result<&SqliteGovernanceLedger, MemoryGovernanceError> {
        self.initialize_mapping().await?;
        self.ledger
            .get_or_try_init(|| async { SqliteGovernanceLedger::new(self.pool.clone()).await })
            .await
            .map_err(Into::into)
    }

    async fn initialize_mapping(&self) -> Result<(), MemoryGovernanceError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memory_proposal_governance (
                proposal_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                content_digest TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn submit(
        &self,
        proposal: &EvolutionProposal,
        session_id: Option<&str>,
        now: DateTime<Utc>,
    ) -> Result<MemoryGovernanceView, MemoryGovernanceError> {
        self.initialize_mapping().await?;
        let request = self.request_for(proposal, session_id, now);
        let digest = request.content_digest.value.clone();
        if let Some(row) = sqlx::query(
            "SELECT request_id, content_digest FROM memory_proposal_governance WHERE proposal_id = ?",
        )
        .bind(&proposal.id)
        .fetch_optional(&self.pool)
        .await?
        {
            let old_request: String = row.get("request_id");
            let old_digest: String = row.get("content_digest");
            if old_digest == digest {
                let state = self.ledger().await?.state(&old_request, now).await?;
                return Ok(self.view(proposal, &request, state, now));
            }
            let ledger = self.ledger().await?;
            if let Ok(old_state) = ledger.state(&old_request, now).await {
                if matches!(
                    old_state.status,
                    ApprovalStatus::Pending | ApprovalStatus::Allowed
                ) {
                    let _ = ledger
                        .revoke(
                            &old_request,
                            old_state.version,
                            &format!("memory-edit-revoke:{}:{digest}", proposal.id),
                            GovernanceSubject {
                                kind: GovernanceSubjectKind::System,
                                id: "memory-governance".into(),
                            },
                            now,
                        )
                        .await;
                }
            }
        }
        let record =
            ApprovalRecord::from_request(&request).map_err(ApprovalLedgerError::Validation)?;
        let state = self
            .ledger()
            .await?
            .submit(
                record,
                &format!("memory-submit:{}", request.correlation.request_id),
                now,
            )
            .await?;
        sqlx::query(
            "INSERT INTO memory_proposal_governance(proposal_id, request_id, content_digest, updated_at)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(proposal_id) DO UPDATE SET
               request_id=excluded.request_id,
               content_digest=excluded.content_digest,
               updated_at=excluded.updated_at",
        )
        .bind(&proposal.id)
        .bind(&request.correlation.request_id)
        .bind(&digest)
        .bind(now.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(self.view(proposal, &request, state, now))
    }

    pub async fn decide(
        &self,
        proposal: &EvolutionProposal,
        expected_version: u64,
        input: MemoryGovernanceDecision<'_>,
    ) -> Result<MemoryGovernanceView, MemoryGovernanceError> {
        let request = self.request_for(proposal, None, proposal.updated_at);
        let current = self.mapped_state(proposal, input.decided_at).await?;
        if current.request.content_digest != request.content_digest {
            return Err(MemoryGovernanceError::StaleProposal);
        }
        if input.decision == Decision::Allow
            && (proposal.risk_level != RiskLevel::Low || input.grant == ApprovalGrant::Unknown)
            && input.grant != ApprovalGrant::Once
        {
            return Err(MemoryGovernanceError::InvalidGrant);
        }
        let receipt = ApprovalReceipt {
            request_id: current.request.correlation.request_id.clone(),
            content_digest: current.request.content_digest.clone(),
            policy_version: current.request.policy_version.clone(),
            capability: Capability::MemoryApply,
            resource: current.request.resource.clone(),
            decision: input.decision,
            decided_by: input.actor,
            decided_at: input.decided_at,
            expires_at: current.request.expires_at,
            grant: input.grant,
        };
        let state = self
            .ledger()
            .await?
            .decide(
                &current.request.correlation.request_id,
                expected_version,
                input.idempotency_key,
                receipt,
            )
            .await?;
        Ok(self.view(proposal, &request, state, input.decided_at))
    }

    pub async fn allowed_receipt(
        &self,
        proposal: &EvolutionProposal,
        now: DateTime<Utc>,
    ) -> Result<(ApprovalState, ApprovalReceipt), MemoryGovernanceError> {
        let state = self.mapped_state(proposal, now).await?;
        if state.request.content_digest != proposal_digest(proposal) {
            return Err(MemoryGovernanceError::StaleProposal);
        }
        if state.status != ApprovalStatus::Allowed {
            return Err(MemoryGovernanceError::ApprovalRequired);
        }
        let receipt = state
            .receipt
            .clone()
            .ok_or(MemoryGovernanceError::ApprovalRequired)?;
        Ok((state, receipt))
    }

    pub async fn consume_once(
        &self,
        request_id: &str,
        expected_version: u64,
        idempotency_key: &str,
        now: DateTime<Utc>,
    ) -> Result<ApprovalState, MemoryGovernanceError> {
        Ok(self
            .ledger()
            .await?
            .consume_once(request_id, expected_version, idempotency_key, now)
            .await?)
    }

    async fn mapped_state(
        &self,
        proposal: &EvolutionProposal,
        now: DateTime<Utc>,
    ) -> Result<ApprovalState, MemoryGovernanceError> {
        self.initialize_mapping().await?;
        let request_id: Option<String> = sqlx::query_scalar(
            "SELECT request_id FROM memory_proposal_governance WHERE proposal_id = ?",
        )
        .bind(&proposal.id)
        .fetch_optional(&self.pool)
        .await?;
        let request_id = request_id.ok_or(MemoryGovernanceError::ApprovalRequired)?;
        Ok(self.ledger().await?.state(&request_id, now).await?)
    }

    fn request_for(
        &self,
        proposal: &EvolutionProposal,
        session_id: Option<&str>,
        now: DateTime<Utc>,
    ) -> ApprovalRequest<()> {
        let digest = proposal_digest(proposal);
        let short_digest = digest.value.chars().take(16).collect::<String>();
        ApprovalRequest {
            correlation: AuditCorrelation {
                request_id: format!("memory-apply:{}:{short_digest}", proposal.id),
                turn_id: format!("proposal:{}", proposal.id),
                session_id: session_id.unwrap_or("workspace").to_string(),
                trace_id: proposal.source_run_id.clone(),
            },
            subject: GovernanceSubject {
                kind: GovernanceSubjectKind::Service,
                id: proposal.created_by.clone(),
            },
            capability: Capability::MemoryApply,
            resource: ResourceScope {
                workspace_id: self.workspace_id.clone(),
                session_id: session_id.map(str::to_string),
                kind: ResourceKind::Memory,
                resource_id: proposal.id.clone(),
                boundary: Some(proposal.target_section.as_str().to_string()),
            },
            risk: risk_class(&proposal.risk_level),
            content_digest: digest,
            policy_version: POLICY_VERSION.into(),
            created_at: now,
            expires_at: now + Duration::hours(REQUEST_TTL_HOURS),
            evidence_refs: proposal
                .evidence_refs
                .iter()
                .map(|evidence| agent_diva_core::governance::EvidenceRef {
                    id: evidence.id.clone(),
                    source: evidence.source.clone(),
                    uri: evidence.uri.clone(),
                    excerpt: evidence.excerpt.clone(),
                    hash: evidence.hash.clone(),
                    created_at: evidence.created_at,
                })
                .collect(),
            payload: (),
        }
    }

    fn view(
        &self,
        proposal: &EvolutionProposal,
        request: &ApprovalRequest<()>,
        state: ApprovalState,
        now: DateTime<Utc>,
    ) -> MemoryGovernanceView {
        let policy = evaluate_policy(
            request,
            &PolicyContext {
                evaluated_at: now,
                autonomy: AutonomyLevel::L1,
                explicit_user_decision: None,
                restrictions: Vec::new(),
                authorizations: state.receipt.iter().cloned().collect(),
            },
        );
        MemoryGovernanceView {
            proposal_id: proposal.id.clone(),
            request_id: state.request.correlation.request_id.clone(),
            request_version: state.version,
            status: state.status,
            policy,
            receipt: state.receipt,
        }
    }
}

pub fn proposal_digest(proposal: &EvolutionProposal) -> ContentDigest {
    let bytes = serde_json::to_vec(&(
        &proposal.id,
        &proposal.proposal_type,
        &proposal.target_section,
        &proposal.proposed_patch,
        &proposal.risk_level,
        &proposal.evidence_refs,
    ))
    .unwrap_or_default();
    let digest = memory_content_digest(&bytes);
    ContentDigest {
        algorithm: DigestAlgorithm::Sha256,
        value: digest.value,
    }
}

fn risk_class(risk: &RiskLevel) -> RiskClass {
    match risk {
        RiskLevel::Low => RiskClass::Low,
        RiskLevel::Medium => RiskClass::Moderate,
        RiskLevel::High => RiskClass::High,
        RiskLevel::Critical => RiskClass::Critical,
    }
}
