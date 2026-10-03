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
  approval_policy?: string
  /** Admitted permission preset per `session/set_permission` (DN-2A). */
  permission_preset?: PermissionPreset | string
  [key: string]: unknown
}

/** Frozen `session/set_permission` preset vocabulary (C2 fixture). */
export type PermissionPreset = 'cautious' | 'smart' | 'trusted'

/** `turn/start` inline image attachment — base64 bytes, sniffed MIME.
 * Max 4 per turn, each ≤5 MiB decoded (internal/attachment, VIVY). */
export interface TurnAttachment {
  name?: string
  mime_type: string
  data: string
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
  network_search?: VivyNetworkSearchResult
  sandbox?: VivySandboxResult
  compaction?: VivyCompactionResult
  http?: VivyHttpResult
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
  network_search?: { provider?: string }
  sandbox?: VivySandboxUpdate
  compaction?: CompactionUpdateParams
  http?: VivyHttpUpdate
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

/** `tools/list` entry — one catalog tool with its effective active flag. */
export interface VivyToolEntry {
  name: string
  description: string
  readonly: boolean
  active: boolean
}

/** `tools/list` result. */
export interface VivyToolsResult {
  tools: VivyToolEntry[]
  /** Effective active set (operator overlay or config fallback). */
  active: string[]
  /** Config-file default used when no overlay was written. */
  config_enabled: string[]
  overlay_written: boolean
}

/** `settings/mcp` entry — endpoint is echoed only when credential-free. */
export interface VivyMcpServer {
  name: string
  transport: string
  endpoint?: string
  command?: string
  args?: string[]
  env_from?: Record<string, string>
  cwd?: string
  /** Host environment variable NAME holding the credential, never a value. */
  auth_env?: string
  resource_bridge: boolean
  auth_env_set: boolean
  enabled: boolean
  env_missing?: string[]
  tool_count: number
  /** ready | unavailable | unconfigured | deferred | inactive | not_compiled */
  state: string
  deferred_reason?: string
  /** Compatibility alias for state; prefer state. */
  status: string
  error?: string
}

/** `settings/mcp` result. */
export interface VivyMcpListResult {
  servers: VivyMcpServer[]
  read_only: boolean
}

/**
 * `settings/mcp/upsert` params — full-replace by name: every field the
 * server should keep must be sent (empty fields wipe stored values).
 */
export interface McpUpsertParams {
  name: string
  transport?: 'http' | 'stdio'
  endpoint?: string
  command?: string
  args?: string[]
  env_from?: Record<string, string>
  cwd?: string
  auth_env?: string
  resource_bridge?: boolean
  enabled?: boolean
}

/** `skills/list` summary entry. */
export interface VivySkillSummary {
  name: string
  description: string
  context?: string
  agent?: string
  model?: string
  user_invocable?: boolean
  origin?: string
  enabled: boolean
  /** Content hash — required as compare-and-swap base for set-enabled. */
  hash: string
  warnings: string[]
}

/** `skills/get` view = summary + document body. */
export interface VivySkillView extends VivySkillSummary {
  content: string
  relative_path: string
  supporting_files: string[]
}

/** `skills/revisions/list` entry — a pending HITL skill mutation. */
export interface VivySkillRevision {
  id: string
  run_id?: string
  skill_name: string
  action: string
  target_path: string
  preview: string
  warnings: string[]
  status: string
  created_at: number
}

/** `skills/marketplace/*` directory entry. */
export interface VivyMarketplaceSkill {
  id: string
  name: string
  source: string
  installs: number
}

/** `skills/marketplace/featured` result. */
export interface VivyMarketplaceFeatured {
  generated_at?: string
  source?: string
  metric?: string
  skills: VivyMarketplaceSkill[]
}

/** `skills/marketplace/install` result. */
export interface VivyMarketplaceInstallResult {
  skill: VivySkillView
  /** created | upgraded | up_to_date */
  outcome: string
  skipped_files?: string[]
  warnings?: string[]
}

/** `settings/get` network_search provider roster entry. */
export interface VivyNetworkSearchProvider {
  name: string
  keyless: boolean
  /** Whether the provider's env credential is present (value never exposed). */
  configured: boolean
  env_key?: string
}

/** `settings/get` network_search section. */
export interface VivyNetworkSearchResult {
  provider: string
  config_provider: string
  providers: VivyNetworkSearchProvider[]
}

/** `settings/get` compaction section (effective + config fallbacks). */
export interface VivyCompactionResult {
  enabled: boolean
  max_tokens: number
  trigger_percent: number
  keep_recent: number
  config_enabled: boolean
  config_max_tokens: number
  config_trigger_percent: number
  config_keep_recent: number
}

/** `settings/update` compaction overlay params. */
export interface CompactionUpdateParams {
  enabled?: boolean
  max_tokens?: number
  trigger_percent?: number
  keep_recent?: number
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

// ---- slice C: channels / cron ----

export interface VivyChannelCapabilities {
  typing: boolean
  edit: boolean
  delete: boolean
  reaction: boolean
  placeholder: boolean
  media: boolean
  media_store: boolean
  webhook: boolean
  listen: boolean
  stream: boolean
  health: boolean
}

export interface VivyChannelHealth {
  ok: boolean
  class?: string
  detail?: string
}

/** channel/inspect entry: process truth from the last StartAll. */
export interface VivyChannelStatus {
  name: string
  capabilities: VivyChannelCapabilities
  health?: VivyChannelHealth | null
  configured: boolean
  enabled: boolean
  allow_from: string[]
  started: boolean
  token_env: string
  token_env_set: boolean
  note: string
}

/** channel/get + channel/update result: document truth folded with the overlay. */
export interface VivyChannelEnvelope {
  name: string
  enabled: boolean
  allow_from: string[]
  token_env: string
  configured: boolean
}

export interface ChannelUpdateParams {
  name: string
  enabled?: boolean
  allow_from?: string[]
  token_env?: string
}

export interface VivyCronSchedule {
  kind: string
  atMs?: number
  everyMs?: number
  expr?: string
  tz?: string
}

export interface VivyCronPayload {
  kind: string
  message: string
  deliver: boolean
  channel?: string
  to?: string
}

export interface VivyCronState {
  nextRunAtMs?: number
  lastRunAtMs?: number
  lastStatus?: string
  lastError?: string
}

export interface VivyCronActiveRun {
  run_id: string
  job_id: string
  startedAtMs: number
  lastHeartbeatAtMs: number
  trigger: string
  cancelable: boolean
}

export interface VivyCronJob {
  id: string
  name: string
  enabled: boolean
  schedule: VivyCronSchedule
  payload: VivyCronPayload
  sessionId?: string
  state: VivyCronState
  deleteAfterRun: boolean
  createdAtMs: number
  updatedAtMs: number
  isRunning: boolean
  activeRun?: VivyCronActiveRun | null
  computedStatus: string
}

export interface VivyCronListResult {
  jobs: VivyCronJob[]
}

/** cron/create + cron/update write params; update adds {id}. */
export interface CronJobWriteParams {
  name: string
  enabled?: boolean
  schedule: VivyCronSchedule
  payload: {
    kind?: string
    message: string
    deliver: boolean
    channel?: string
    to?: string
  }
  delete_after_run?: boolean
}

/** `stats/tokens` coverage record (D3). observed_calls partitions as
 * completed_with_usage + partial_usage_calls + missing_usage_calls +
 * active_calls; reported_calls may also count active calls and is not
 * part of that partition. `request_count` on the aggregates remains
 * "usage reports" — reported calls plus legacy records — never total
 * billed requests. */
export interface VivyUsageCoverage {
  /** empty | complete | partial | legacy */
  state: string
  observed_calls: number
  completed_with_usage: number
  reported_calls: number
  missing_usage_calls: number
  partial_usage_calls: number
  active_calls: number
  legacy_usage_records: number
  unknown_buckets: string[]
  /** Always false: provider-internal retries are not journaled. */
  hidden_retries_observable: boolean
}

/** stats/tokens result snapshot. */
export interface VivyTokenUsageTotal {
  total_input: number
  total_output: number
  total_tokens: number
  total_reasoning: number
  total_cached: number
  request_count: number
  total_cost_usd: number
  cost_known: boolean
}

export interface VivyTokenModelShare {
  model: string
  percentage: number
  total_tokens: number
  cost_usd: number
  cost_known: boolean
  coverage: VivyUsageCoverage
}

export interface VivyTokenProviderGroup {
  key: string
  total_tokens: number
  request_count: number
}

export interface VivyTokenTimelinePoint {
  time_bucket: string
  label: string
  total_input: number
  total_output: number
  total_tokens: number
}

export interface VivyTokenSessionUsage {
  id: string
  title: string
  model: string
  request_count: number
  total_input: number
  total_output: number
  total_tokens: number
  cost_usd: number
  cost_known: boolean
  coverage: VivyUsageCoverage
}

export interface VivyTokenUsageSnapshot {
  period: string
  scope: string
  projection_version: number
  coverage: VivyUsageCoverage
  total: VivyTokenUsageTotal
  models: VivyTokenModelShare[]
  providers: VivyTokenProviderGroup[]
  timeline: VivyTokenTimelinePoint[]
  sessions: VivyTokenSessionUsage[]
}

/** `settings/get` sandbox section: effective values + config fallbacks. */
export interface VivySandboxResult {
  /** cautious | smart | trusted | custom */
  default_preset: string
  config_default_preset: string
  deny_private_ips: boolean
  allowed_domains: string[]
  workspace_root: string
  execute_allowed_commands: string[]
  /** 0 = timed auto-approval disabled. */
  approval_timeout_seconds: number
  config_approval_timeout_seconds: number
  /** Hard expiration bounding the approval review window. */
  approval_expiration_seconds: number
}

/** `settings/update` sandbox overlay; absent fields keep prior value. */
export interface VivySandboxUpdate {
  default_preset?: string
  deny_private_ips?: boolean
  allowed_domains?: string[]
  approval_timeout_seconds?: number
}

/** `settings/get` http section. */
export interface VivyHttpResult {
  allowed_hosts: string[]
  timeout_seconds: number
  config_allowed_hosts: string[]
  config_timeout_seconds: number
  overlay_set: boolean
}

/** `settings/update` http overlay. */
export interface VivyHttpUpdate {
  allowed_hosts?: string[]
  timeout_seconds?: number
}
