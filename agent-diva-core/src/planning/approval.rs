//! Revision-bound plan submission and approval contracts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ids::PlanId;

/// Determines whether an approval creates execution TODO items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoPolicy {
    Never,
    Optional,
    Always,
}

impl TodoPolicy {
    /// Returns whether this request must materialize TODO items.
    pub fn materializes(self, requested: bool) -> bool {
        matches!(self, Self::Always) || (matches!(self, Self::Optional) && requested)
    }
}

/// Metadata required before a plan can be submitted for approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanSubmission {
    pub scope: String,
    pub verification_method: String,
    pub open_question_handling: String,
}

/// User-originated request to approve one frozen plan revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub expected_revision: i64,
    #[serde(skip_deserializing, default = "default_audit_approver")]
    pub approved_by: String,
    pub todo_policy: TodoPolicy,
    pub materialize_todos: bool,
}

/// GUI callers never choose the audit subject.  It is an internal provenance
/// marker for a local desktop approval.
fn default_audit_approver() -> String {
    "desktop-ui".to_string()
}

/// Immutable audit record created by a successful approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalReceipt {
    pub plan_id: PlanId,
    pub revision: i64,
    pub approved_by: String,
    pub approved_at: DateTime<Utc>,
    pub todo_policy: TodoPolicy,
    pub todos_materialized: bool,
}
