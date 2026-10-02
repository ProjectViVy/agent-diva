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

/**
 * Settings/providers surface: verified against `internal/rpc/control.go`
 * (settingsResult/providersResult/catalogEntryResult/providerEntryResult and
 * the settings/* handler block). Credentials are write-only on the wire:
 * `api_key` goes in, `api_key_set` comes out — the backend never echoes a key.
 */
export interface VivyProviderEntryResult {
  id: string
  display_name: string
  /** Adapter family or legacy vendor alias
   * (openai-completions|openai-responses|anthropic-messages|deepseek|openai|anthropic). */
  bundle: string
  base_url: string
  default_model: string
  models: string[]
  api_key_set: boolean
  [key: string]: unknown
}

export interface VivyCatalogEndpoint {
  adapter: string
  base_url: string
  default_model: string
  models: string[]
  /** False for a sealed adapter this Generation cannot construct. */
  executable: boolean
  state: string
}

export interface VivyCatalogEntry {
  vendor: string
  display_name: string
  endpoints: VivyCatalogEndpoint[]
}

export interface VivyProviderProfileStatus {
  id: string
  adapter_family: string
  endpoint_class: string
  model_ids: string[]
  state:
    | 'COMPILED'
    | 'UNCONFIGURED'
    | 'READY'
    | 'UNAVAILABLE'
    | 'DEFERRED-INDEFINITE'
    | string
}

export interface VivyProvidersResult {
  entries: VivyProviderEntryResult[]
  catalog: VivyCatalogEntry[]
  profiles: VivyProviderProfileStatus[]
  active_provider: string
  active_model: string
  active_base_url: string
  read_only: boolean
  frozen: boolean
  config_provider: string
  config_model: string
  [key: string]: unknown
}

export interface VivySettingsResult {
  /** Active bundle name or empty for the config default. */
  provider: string
  default_model: string
  base_url: string
  /** Presence flag only — the stored key value is never returned. */
  api_key_set: boolean
  frozen: boolean
  read_only: boolean
  config_provider: string
  config_model: string
  config_execute_max_timeout_seconds?: number
  execute_max_timeout_seconds?: number
  provider_profiles?: VivyProviderProfileStatus[]
  network_search?: Record<string, unknown>
  sandbox?: Record<string, unknown>
  compaction?: Record<string, unknown>
  http?: Record<string, unknown>
  locale?: string
  generation_locale?: string
  workspace_locale?: string
  locale_read_only?: boolean
  [key: string]: unknown
}

/** `settings/update` merge-write params; every field optional. */
export interface SettingsUpdateParams {
  provider?: string
  default_model?: string
  base_url?: string
  /** Write-only credential for the global key overlay. */
  api_key?: string
  network_search?: { provider?: string; [key: string]: unknown }
  sandbox?: Record<string, unknown>
  compaction?: Record<string, unknown>
  http?: Record<string, unknown>
  execute_max_timeout_seconds?: number
  [key: string]: unknown
}

/**
 * `settings/providers/upsert` params. On update-by-id the backend keeps any
 * stored field the patch leaves empty (empty api_key never wipes the
 * credential); a create requires bundle+base_url.
 */
export interface ProviderUpsertParams {
  /** Update an existing entry when set; omitted creates a `custom-*` entry. */
  id?: string
  display_name?: string
  bundle?: string
  base_url?: string
  default_model?: string
  models?: string[]
  /** Write-only credential; persisted but never returned. */
  api_key?: string
}

/**
 * `settings/providers/refresh` params: `{id}` refreshes a registry entry;
 * `{bundle, base_url, display_name?, default_model?}` clones a catalog
 * endpoint into the registry and refreshes it (OpenAI-compatible only —
 * the backend live-fetches GET /models).
 */
export interface ProviderRefreshParams {
  id?: string
  bundle?: string
  base_url?: string
  display_name?: string
  default_model?: string
}

/**
 * `settings/model/select` params. `provider` accepts a registry bundle, a
 * legacy vendor alias, or the config default; the target must be the config
 * default, a registry entry model, or a catalog endpoint model — otherwise
 * the backend rejects with InvalidParams.
 */
export interface ModelSelectParams {
  provider: string
  model: string
  base_url?: string
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
