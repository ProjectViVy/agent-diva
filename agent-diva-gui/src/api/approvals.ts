import type { ReviewItem } from './vivy/contracts';

export type ApprovalDomain = 'command' | 'plan';
export type ApprovalStatus = 'pending' | 'allowed' | 'denied' | 'revoked' | 'consumed' | 'expired';
export type ApprovalReasonCode =
  | 'approval_not_found' | 'approval_version_conflict' | 'approval_idempotency_conflict'
  | 'approval_expired' | 'approval_already_resolved' | 'approval_already_consumed'
  | 'approval_digest_mismatch' | 'approval_invalid_grant' | 'approval_invalid_transition'
  | 'approval_payload_unavailable' | 'approval_persistence_failed'
  | 'approval_required_noninteractive' | 'approval_queue_unavailable' | 'approval_outcome_unknown'
  | 'approval_denied' | 'approval_revoked' | 'approval_invalid_cursor'
  | 'approval_invalid_body' | 'approval_invalid_query';

export interface ApprovalResource {
  workspace_id: string;
  session_id: string | null;
  kind: string;
  resource_id: string;
  boundary: string | null;
}

export interface ApprovalView {
  request_id: string;
  version: number;
  domain: ApprovalDomain;
  capability: string;
  resource: ApprovalResource;
  risk: string;
  status: ApprovalStatus;
  created_at: string;
  expires_at: string;
  subject: { kind: string; id: string };
  evidence: unknown[];
  receipt: unknown | null;
  reason_code: ApprovalReasonCode | null;
  actions: string[];
  presentation?: Record<string, unknown> | null;
}

export type ApprovalGrant = 'once' | 'session' | 'rule';

export interface UnifiedApprovalApiError {
  status: number;
  reason_code: ApprovalReasonCode;
  message?: string;
}

const reasons = new Set([
  'approval_not_found', 'approval_version_conflict', 'approval_idempotency_conflict',
  'approval_expired', 'approval_already_resolved', 'approval_already_consumed',
  'approval_digest_mismatch', 'approval_invalid_grant', 'approval_invalid_transition',
  'approval_payload_unavailable', 'approval_persistence_failed',
  'approval_required_noninteractive', 'approval_queue_unavailable', 'approval_outcome_unknown',
  'approval_denied', 'approval_revoked', 'approval_invalid_cursor',
  'approval_invalid_body', 'approval_invalid_query',
]);
const optionalReason = (value: unknown): value is ApprovalReasonCode | null =>
  value === null || (typeof value === 'string' && reasons.has(value));
const object = (value: unknown): value is Record<string, unknown> =>
  typeof value === 'object' && value !== null && !Array.isArray(value);

// ---------------------------------------------------------------------------
// VIVY review adapter (DN-2): review/list rows of kind "approval" map onto the
// drawer-facing ApprovalView. Pending state is authoritative from
// review/list snapshots plus tool.approval_* events — no optimistic cache.
// ---------------------------------------------------------------------------

const REVIEW_STATUS_TO_APPROVAL: Record<string, ApprovalStatus> = {
  pending: 'pending',
  approved: 'allowed',
  denied: 'denied',
  cancelled: 'revoked',
  expired: 'expired',
  answered: 'consumed',
  stale: 'revoked',
};

function reviewRecord(value: unknown): Record<string, unknown> | null {
  return object(value) ? value : null;
}

function reviewString(value: unknown): string {
  return typeof value === 'string' ? value : '';
}

function isoFromUnixSeconds(value: unknown): string {
  return typeof value === 'number' && Number.isFinite(value)
    ? new Date(value * 1000).toISOString()
    : '';
}

/** Map one VIVY review item to the ApprovalCenter drawer view. */
export function reviewToApprovalView(review: ReviewItem): ApprovalView | null {
  if (review.kind !== 'approval') return null;
  const action = reviewRecord(review.action) ?? {};
  const args = reviewRecord(review.arguments) ?? {};
  const riskFindings = Array.isArray(review.risk_findings)
    ? review.risk_findings.filter((f): f is string => typeof f === 'string')
    : [];
  const evidence: unknown[] = [];
  if (review.prompt) evidence.push({ prompt: review.prompt });
  if (review.preview) evidence.push({ preview: review.preview });
  if (Object.keys(args).length > 0) evidence.push({ arguments: args });
  const reason = review.decision_reason || review.stale_reason || review.error || '';
  return {
    request_id: review.id,
    version: 0,
    domain: 'command',
    capability: review.tool_name || 'tool',
    resource: {
      workspace_id: '',
      session_id: review.session_id || null,
      kind: reviewString(action.kind) || 'tool_call',
      resource_id: review.tool_call_id || '',
      boundary: reviewString(action.boundary) || null,
    },
    risk: riskFindings.length > 0 ? 'high' : 'medium',
    status: REVIEW_STATUS_TO_APPROVAL[review.status] ?? 'pending',
    created_at: isoFromUnixSeconds(review.created_at),
    expires_at: isoFromUnixSeconds(review.expires_at),
    subject: { kind: reviewString(action.subject_kind) || 'actor', id: review.actor || '' },
    evidence,
    receipt: null,
    reason_code: reason
      ? (optionalReason(reason) ? reason : 'approval_invalid_transition')
      : null,
    actions: review.status === 'pending' ? ['approve', 'deny'] : [],
    presentation: review.preview ? { markdown: review.preview, risk_findings: riskFindings } : null,
  };
}

/** Map a VIVY review list to ApprovalView[], dropping non-approval rows. */
export function reviewsToApprovalViews(reviews: ReviewItem[]): ApprovalView[] {
  const out: ApprovalView[] = [];
  for (const review of reviews) {
    const view = reviewToApprovalView(review);
    if (view) out.push(view);
  }
  return out;
}
