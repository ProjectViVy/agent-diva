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

/** A backend notification forwarded on `vivy:event` ({kind:'vivy'}). */
export interface VivyNotification {
  method: string
  params: unknown
}

/** Wire event frame delivered by the shell transport. */
export type WireEvent =
  | { kind: 'vivy'; method: string; params: unknown }
  | { kind: 'bridge'; status: 'gap' | 'lost' }
