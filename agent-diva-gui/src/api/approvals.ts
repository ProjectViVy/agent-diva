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

export interface ApprovalEventView {
  event_id: string;
  cursor: string;
  request_id: string;
  version: number;
  domain: ApprovalDomain;
  status: ApprovalStatus;
  reason_code: ApprovalReasonCode | null;
  reason: ApprovalReasonCode | null;
  occurred_at: string;
  correlation: Record<string, unknown> | null;
}

export interface ApprovalListPage {
  approvals: ApprovalView[];
  next_cursor: string | null;
}

export type ApprovalGrant = 'once' | 'session' | 'rule';
export type ApprovalDecision = 'allow' | 'deny';

export interface UnifiedApprovalApiError {
  status: number;
  reason_code: ApprovalReasonCode;
  message?: string;
}

const domains = new Set(['command', 'plan']);
const statuses = new Set(['pending', 'allowed', 'denied', 'revoked', 'consumed', 'expired']);
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

export function isApprovalView(value: unknown): value is ApprovalView {
  if (!object(value)) return false;
  return typeof value.request_id === 'string'
    && Number.isInteger(value.version)
    && domains.has(String(value.domain))
    && typeof value.capability === 'string'
    && object(value.resource)
    && typeof value.risk === 'string'
    && statuses.has(String(value.status))
    && typeof value.created_at === 'string'
    && typeof value.expires_at === 'string'
    && object(value.subject)
    && Array.isArray(value.evidence)
    && optionalReason(value.reason_code)
    && Array.isArray(value.actions)
    && value.actions.every((action) => typeof action === 'string');
}

export function isApprovalEventView(value: unknown): value is ApprovalEventView {
  if (!object(value)) return false;
  return typeof value.event_id === 'string'
    && typeof value.cursor === 'string'
    && typeof value.request_id === 'string'
    && Number.isInteger(value.version)
    && domains.has(String(value.domain))
    && statuses.has(String(value.status))
    && optionalReason(value.reason_code)
    && optionalReason(value.reason)
    && typeof value.occurred_at === 'string'
    && (value.correlation === null || object(value.correlation));
}

export function isApprovalListPage(value: unknown): value is ApprovalListPage {
  return object(value)
    && Array.isArray(value.approvals)
    && value.approvals.every(isApprovalView)
    && (value.next_cursor === null || typeof value.next_cursor === 'string');
}

export class ApprovalEventGuard {
  private readonly seen = new Set<string>();

  constructor(private readonly capacity = 2048) {}

  accept(event: ApprovalEventView, knownVersion: number): boolean {
    if (this.seen.has(event.event_id) || event.version < knownVersion) return false;
    this.seen.add(event.event_id);
    if (this.seen.size > this.capacity) {
      const oldest = this.seen.values().next().value;
      if (oldest) this.seen.delete(oldest);
    }
    return true;
  }
}
