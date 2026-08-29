/**
 * TypeScript view of the Super Channel Fabric Neuro-Link v1 contract.
 *
 * The JSON Schema under `schemas/neuro-link/v1/` is authoritative. These
 * types are intentionally not imported by the current chat UI until C4.
 */

export const NEURO_LINK_PROTOCOL_V1 = 'neuro-link/v1' as const
export const CHANNEL_SCHEMA_VERSION_V1 = 1 as const
export const JSON_RPC_VERSION = '2.0' as const

export type ChannelDirection = 'ingress' | 'egress' | 'internal_projection'
export type ChannelOrigin = 'external_user' | 'owner_frontend' | 'runtime'

export interface ChannelAddress {
  channel: string
  account_id?: string
  sender_id?: string
  chat_id: string
  thread_id?: string
}

export interface Correlation {
  session_key: string
  request_id?: string
  trace_id?: string
  message_id?: string
  reply_to?: string
  sequence?: number
}

export interface CursorV1 {
  stream: string
  sequence: number
}

export interface AttachmentRef {
  uri: string
  media_type: string
  size_bytes: number
  sha256: string
  file_name?: string
}

export type ContentPart =
  | { kind: 'text'; text: string }
  | { kind: 'markdown'; markdown: string }
  | { kind: 'image'; attachment: AttachmentRef }
  | { kind: 'audio'; attachment: AttachmentRef; transcript?: string }
  | { kind: 'video'; attachment: AttachmentRef }
  | { kind: 'file'; attachment: AttachmentRef }
  | { kind: 'location'; latitude: number; longitude: number; label?: string }
  | { kind: 'card'; schema: string; body: Record<string, unknown> }
  | { kind: 'reference'; uri: string; title?: string; media_type?: string }

export type TypingState = 'started' | 'stopped' | 'listening'
export type StreamPhase = 'started' | 'delta' | 'finalized' | 'cancelled' | 'failed'
export type ReactionOperation = 'add' | 'remove'
export type DeliveryStatus = 'accepted' | 'delivered' | 'rejected' | 'failed' | 'unsupported'
export type ChannelHealthStatus = 'healthy' | 'degraded' | 'down' | 'unknown'

export interface DeliveryReceipt {
  status: DeliveryStatus
  channel: string
  chat_id: string
  platform_message_id?: string
  thread_id?: string
  retry_after_ms?: number
  error_code?: string
  diagnosis?: string
}

export type ChannelPayloadV1 =
  | { kind: 'message'; parts: ContentPart[]; subject?: string; locale?: string }
  | { kind: 'typing'; state: TypingState }
  | { kind: 'stream'; phase: StreamPhase; parts: ContentPart[] }
  | { kind: 'reaction'; operation: ReactionOperation; emoji: string }
  | { kind: 'delete'; target_message_id: string }
  | { kind: 'delivery'; receipt: DeliveryReceipt }
  | { kind: 'health'; status: ChannelHealthStatus; diagnosis?: string }
  | { kind: 'control'; operation: string; body: Record<string, unknown> }
  | { kind: 'presentation'; event: string; body: Record<string, unknown> }
  | { kind: 'gap'; last_durable_cursor?: CursorV1; reason: string }

export interface ChannelEnvelopeV1 {
  schema_version: typeof CHANNEL_SCHEMA_VERSION_V1
  envelope_id: string
  occurred_at: string
  direction: ChannelDirection
  address: ChannelAddress
  correlation: Correlation
  origin: ChannelOrigin
  payload: ChannelPayloadV1
  extensions: Record<string, unknown>
}

export type RpcId = string | number

export interface RpcRequestV1<Params> {
  jsonrpc: typeof JSON_RPC_VERSION
  id: RpcId
  method: string
  params: Params
}

export interface RpcNotificationV1<Params> {
  jsonrpc: typeof JSON_RPC_VERSION
  method: string
  params: Params
}

export interface RpcResponseV1<Result> {
  jsonrpc: typeof JSON_RPC_VERSION
  id: RpcId
  result: Result
}

export type ProtocolErrorCode =
  | 'invalid_request'
  | 'invalid_params'
  | 'method_not_found'
  | 'internal_error'
  | 'protocol_version_mismatch'
  | 'cursor_out_of_range'
  | 'cursor_invalid'
  | 'unsupported_capability'
  | 'frame_too_large'
  | 'attachment_too_large'

export interface RpcErrorV1 {
  jsonrpc: typeof JSON_RPC_VERSION
  id: RpcId
  error: {
    code: ProtocolErrorCode
    message: string
    data?: Record<string, unknown>
  }
}

export interface ProtocolHelloParams {
  protocol: typeof NEURO_LINK_PROTOCOL_V1
  schema_version: typeof CHANNEL_SCHEMA_VERSION_V1
  frontend_instance_id: string
  capabilities: string[]
}

export interface ProtocolHelloResult {
  protocol: typeof NEURO_LINK_PROTOCOL_V1
  schema_version: typeof CHANNEL_SCHEMA_VERSION_V1
  capabilities: string[]
}

export interface SessionOpenParams {
  session_key: string
  durable_cursor?: CursorV1
}

export interface TurnStartParams {
  session_key: string
  thread_id?: string
  parts: ContentPart[]
  client_message_id?: string
  subject?: string
  locale?: string
}

export interface TurnCancelParams {
  session_key: string
  request_id: string
}

export interface EventAckParams {
  session_key: string
  cursor: CursorV1
}

export interface StateResumeParams {
  session_key: string
  cursor: CursorV1
}
