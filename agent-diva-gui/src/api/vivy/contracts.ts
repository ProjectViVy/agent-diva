/**
 * Frozen VIVY wire contracts (DN-0 fixture `core-rpc.json`, capture on the
 * pinned VIVY `feat/diva-embedded`). Shapes below are the verified fields;
 * payloads may carry additional backend keys which must pass through
 * untouched — never narrow or rename a backend field.
 */

/** Bridge error body emitted by the shell (kind+code+message+data). */
export interface BridgeErrorBody {
  kind:
    | 'invalid_input'
    | 'incompatible_abi'
    | 'closed'
    | 'already_initialized'
    | 'transport_lost'
    | 'timeout'
    | 'event_gap'
    | 'internal'
    | 'load_failed'
    | 'rpc'
  /** VIVY RPC error code when the backend rejected the call (e.g. -32004). */
  code: number
  message: string
  data?: unknown
}

export class VivyCallError extends Error {
  readonly kind: BridgeErrorBody['kind']
  readonly code: number
  readonly data?: unknown
  /**
   * True when the request was a mutating call and its outcome is unknown
   * (timeout/transport loss after the call was submitted). Such calls are
   * never auto-retried; the caller must reconcile through reads.
   */
  readonly unknownOutcome: boolean

  constructor(body: BridgeErrorBody, unknownOutcome = false) {
    super(body.message)
    this.name = 'VivyCallError'
    this.kind = body.kind
    this.code = body.code
    this.data = body.data
    this.unknownOutcome = unknownOutcome
  }
}

export interface InitializeResult {
  protocol_version: string
  capabilities: string[]
}

export interface VivySession {
  id: string
  title: string
  created_at: number
  updated_at: number
  sandbox_mode?: string
  [key: string]: unknown
}

export interface SessionMessage {
  id: string
  run_id?: string
  role: 'user' | 'assistant' | 'system' | string
  content: string
  created_at: number
  [key: string]: unknown
}

export interface SessionListResult {
  sessions: VivySession[]
}

export interface SessionGetResult {
  session: VivySession
  messages: SessionMessage[]
}

export interface SessionMessagesResult {
  messages: SessionMessage[]
}

export interface TurnStartResult {
  run_id: string
  status: 'accepted' | string
}

export type RunStatus = 'active' | 'completed' | 'failed' | 'cancelled' | string

export interface RunGetResult {
  id: string
  session_id: string
  status: RunStatus
  created_at: number
  [key: string]: unknown
}

export interface RunSubscribeResult {
  subscription_id: string
  run_id: string
  after_seq: number
}

export interface RunEvent {
  run_id: string
  seq: number
  type: string
  created_at: number
  payload_version: number
  payload: Record<string, unknown>
}

export interface RunLogResult {
  events: RunEvent[]
}

/** params of the `run/event` notification. */
export interface RunEventParams {
  event: RunEvent
}

export interface Approval {
  id: string
  run_id: string
  tool_call_id: string
  decision: 'pending' | 'approved' | 'denied' | string
  expires_at?: number
  [key: string]: unknown
}

export interface ApprovalListResult {
  approvals: Approval[]
}

export interface Question {
  id: string
  run_id?: string
  [key: string]: unknown
}

export interface QuestionListResult {
  questions: Question[]
}

/**
 * Review surface: verified against the pinned VIVY RPC source
 * (`internal/rpc/control.go` review handlers, `internal/domain/review.go`).
 * Not part of the DN-0 transcript — marked source-verified.
 */
export type ReviewKind = 'approval' | 'question' | string
export type ReviewStatus =
  | 'pending'
  | 'approved'
  | 'denied'
  | 'answered'
  | 'cancelled'
  | 'expired'
  | 'stale'
  | string

export interface ReviewItem {
  id: string
  kind: ReviewKind
  status: ReviewStatus
  session_id?: string
  session_title?: string
  run_id?: string
  tool_call_id?: string
  tool_name?: string
  source?: string
  actor?: string
  created_at?: number
  expires_at?: number
  decided_at?: number
  action?: string
  target?: string
  precondition_hash?: string
  preview?: string
  risk_findings?: string[]
  arguments?: unknown
  prompt?: string
  decision_reason?: string
  stale_reason?: string
  error?: string
  [key: string]: unknown
}

export interface ReviewListParams {
  kind?: string
  status?: string
  session_id?: string
  limit?: number
}

export interface ReviewListResult {
  reviews: ReviewItem[]
}

/** `review/respond` actions: approval → approve|deny; question → answer|cancel. */
export interface ReviewRespondParams {
  review_id: string
  action: 'approve' | 'deny' | 'answer' | 'cancel' | string
  decision?: string
  answer?: string
  reason?: string
}

/**
 * Session todos: verified against `internal/rpc/control.go` listTodos and
 * `internal/domain` Todo. `session/todo/update` accepts
 * pending|in_progress|completed|cancelled.
 */
export interface SessionTodo {
  id: string
  session_id: string
  subject: string
  description?: string
  status: 'pending' | 'in_progress' | 'completed' | 'cancelled' | string
  blocks: string[]
  blocked_by: string[]
  [key: string]: unknown
}

export interface SessionTodosResult {
  todos: SessionTodo[]
}

/**
 * Work/plan surface: verified against `internal/rpc/work_control.go`
 * (workStateView) and `internal/domain/work_control.go`.
 */
export interface WorkPlan {
  active: boolean
  submission_id: string
  markdown: string
  review_status: 'none' | 'pending' | 'accepted' | 'rejected' | 'cancelled' | string
  feedback?: string
  origin_run_id?: string
  origin_tool_call_id?: string
}

export interface WorkGoal {
  id: string
  revision: number
  objective: string
  phase: string
  max_rounds: number
  rounds_started: number
  reason?: string
  evidence_run_id?: string
}

export interface SessionWorkResult {
  session_id: string
  version: number
  plan: WorkPlan
  activation: string
  current_run_id?: string
  goal?: WorkGoal
  process_epoch?: string
}

/** `plan/get` result is the same plan view as `session/work`. */
export type PlanGetResult = WorkPlan

export interface PlanDecideParams {
  session_id: string
  request_id: string
  submission_id: string
  action: 'revise' | 'execute_once' | 'start_goal'
  feedback?: string
  objective?: string
  max_rounds?: number
  goal_id?: string
  expected_version?: number
  reason?: string
}

export interface WorkCommitResult {
  work: SessionWorkResult
  event: { seq: number; kind: string; request_id: string; created_at: number }
  replayed: boolean
}

export interface SessionWorkSubscribeResult {
  subscription_id: string
  session_id: string
  after_seq: number
  watermark_seq: number
  process_epoch?: string
}

export interface WorkEvent {
  seq: number
  kind: string
  request_id: string
  created_at: number
}

/** params of the `session/work/event` notification. */
export interface WorkEventParams {
  subscription_id: string
  event: WorkEvent
  process_epoch?: string
  work_version: number
}

/** A backend notification forwarded on `vivy:event` ({kind:'vivy'}). */
export interface VivyNotification {
  method: string
  params: unknown
}

/** Wire event frame delivered by the shell transport. */
export type WireEvent =
  | { kind: 'vivy'; method: string; params: unknown }
  | { kind: 'bridge'; status: 'gap' | 'lost' }
