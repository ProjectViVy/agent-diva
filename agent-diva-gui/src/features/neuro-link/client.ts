import { invoke } from '@tauri-apps/api/core'
import {
  CHANNEL_SCHEMA_VERSION_V1,
  JSON_RPC_VERSION,
  NEURO_LINK_PROTOCOL_V1,
  type ChannelEnvelopeV1,
  type CursorV1,
  type EventAckParams,
  type EventAckResultV1,
  type EnvelopeNotificationParams,
  type ProtocolHelloParams,
  type ProtocolHelloResult,
  type RpcErrorV1,
  type RpcId,
  type RpcResponseV1,
  type SessionOpenParams,
  type SessionOpenResultV1,
  type StateResumeParams,
  type StateSyncResultV1,
  type TurnCancelParams,
  type TurnCancelResultV1,
  type TurnStartParams,
  type TurnStartResultV1,
} from '../../protocol/neuro-link-v1'

export type NeuroLinkConnectionStatus = 'idle' | 'connecting' | 'connected' | 'error' | 'closed'
export type ProjectionSource = 'live' | 'replay'

export interface ProjectionEnvelopeEvent {
  method: string
  cursor: CursorV1
  envelope: ChannelEnvelopeV1
  source: ProjectionSource
}

export interface GatewayStatus {
  port: number
  running: boolean
}

export interface WebSocketEventLike {
  data?: unknown
}

export interface WebSocketLike {
  addEventListener(type: string, listener: (event: WebSocketEventLike) => void): void
  send(data: string): void
  close(code?: number, reason?: string): void
}

export type WebSocketFactory = (url: string) => WebSocketLike
export type GatewayEndpointProvider = () => string | Promise<string>

export interface NeuroLinkClientOptions {
  endpoint: GatewayEndpointProvider
  frontendInstanceId: string
  capabilities?: string[]
  webSocketFactory?: WebSocketFactory
  requestTimeoutMs?: number
  onProjection?: (event: ProjectionEnvelopeEvent) => void | Promise<void>
  onStatusChange?: (status: NeuroLinkConnectionStatus) => void
}

export class NeuroLinkProtocolError extends Error {
  readonly code: string
  readonly data?: Record<string, unknown>

  constructor(error: RpcErrorV1['error']) {
    super(error.message)
    this.name = 'NeuroLinkProtocolError'
    this.code = error.code
    this.data = error.data
  }
}

interface PendingRequest {
  resolve: (value: unknown) => void
  reject: (reason?: unknown) => void
  timer: ReturnType<typeof setTimeout>
}

const DEFAULT_CAPABILITIES = [
  'bounded_frames',
  'durable_projection',
  'idempotent_commands',
  'state_sync',
  'owner_turn_context',
  'presentation_events',
]

const DEFAULT_REQUEST_TIMEOUT_MS = 15_000

export function defaultWebSocketFactory(url: string): WebSocketLike {
  return new WebSocket(url)
}

export async function tauriGatewayEndpoint(): Promise<string> {
  const status = await invoke<GatewayStatus>('get_gateway_status')
  if (!status.running || !Number.isInteger(status.port) || status.port <= 0) {
    throw new Error('embedded gateway is not running')
  }
  return `ws://127.0.0.1:${status.port}/api/neuro-link/v1/ws`
}

export function makeFrontendInstanceId(storage: Storage | undefined = typeof localStorage === 'undefined' ? undefined : localStorage): string {
  const key = 'agent-diva.neuro-link.frontend-instance-id'
  let existing: string | undefined
  try {
    existing = storage?.getItem(key)?.trim()
  } catch {
    // A restricted browser profile can deny localStorage reads; generate an
    // in-memory identity for this frontend process instead.
  }
  if (existing) return existing
  const value = typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `frontend-${Date.now()}-${Math.random().toString(36).slice(2)}`
  try {
    storage?.setItem(key, value)
  } catch {
    // A restricted browser profile can deny localStorage; the in-memory ID is
    // still valid for the lifetime of this frontend process.
  }
  return value
}

export function cursorStorageKey(workspaceRoot: string, sessionKey: string): string {
  return `agent-diva.neuro-link.cursor:${workspaceRoot}:${sessionKey}`
}

export function readPersistedCursor(storage: Storage | undefined, workspaceRoot: string, sessionKey: string): CursorV1 {
  const fallback = { stream: sessionKey, sequence: 0 }
  if (!storage) return fallback
  try {
    const raw = storage.getItem(cursorStorageKey(workspaceRoot, sessionKey))
    if (!raw) return fallback
    const parsed = JSON.parse(raw) as Partial<CursorV1>
    if (parsed.stream === sessionKey && Number.isInteger(parsed.sequence) && (parsed.sequence ?? -1) >= 0) {
      return { stream: sessionKey, sequence: parsed.sequence as number }
    }
  } catch {
    // A malformed local cursor is equivalent to the initial cursor.
  }
  return fallback
}

export function persistCursor(storage: Storage | undefined, workspaceRoot: string, cursor: CursorV1): void {
  if (!storage || cursor.sequence < 0 || !cursor.stream) return
  try {
    storage.setItem(cursorStorageKey(workspaceRoot, cursor.stream), JSON.stringify(cursor))
  } catch {
    // Cursor persistence is best effort; an unavailable/quota-limited storage
    // must not tear down an otherwise healthy live projection connection.
  }
}

export class NeuroLinkClient {
  private readonly options: Required<Pick<NeuroLinkClientOptions, 'capabilities' | 'requestTimeoutMs' | 'webSocketFactory'>>
  private socket: WebSocketLike | null = null
  private pending = new Map<string, PendingRequest>()
  private requestSequence = 0
  private closedByCaller = false
  private status: NeuroLinkConnectionStatus = 'idle'
  private lastAppliedSequence = new Map<string, number>()
  private projectionQueue: Promise<void> = Promise.resolve()

  constructor(private readonly config: NeuroLinkClientOptions) {
    this.options = {
      capabilities: config.capabilities ?? DEFAULT_CAPABILITIES,
      requestTimeoutMs: config.requestTimeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS,
      webSocketFactory: config.webSocketFactory ?? defaultWebSocketFactory,
    }
  }

  get connectionStatus(): NeuroLinkConnectionStatus {
    return this.status
  }

  async connect(): Promise<ProtocolHelloResult> {
    if (this.status === 'connected' && this.socket) {
      throw new Error('Neuro-Link client is already connected')
    }
    this.closedByCaller = false
    this.setStatus('connecting')
    const endpoint = await this.config.endpoint()
    const socket = this.options.webSocketFactory(endpoint)
    this.socket = socket
    this.attachSocket(socket)
    await this.waitForOpen(socket)
    const hello: ProtocolHelloParams = {
      protocol: NEURO_LINK_PROTOCOL_V1,
      schema_version: CHANNEL_SCHEMA_VERSION_V1,
      frontend_instance_id: this.config.frontendInstanceId,
      capabilities: this.options.capabilities,
    }
    const result = await this.request<ProtocolHelloResult>('protocol/hello', hello)
    if (result.protocol !== NEURO_LINK_PROTOCOL_V1 || result.schema_version !== CHANNEL_SCHEMA_VERSION_V1) {
      this.failConnection(new Error('Neuro-Link protocol/schema mismatch'))
      throw new Error('Neuro-Link protocol/schema mismatch')
    }
    this.setStatus('connected')
    return result
  }

  async openSession(params: SessionOpenParams): Promise<SessionOpenResultV1> {
    const result = await this.request<SessionOpenResultV1>('session/open', params)
    const cursor = params.durable_cursor ?? { stream: params.session_key, sequence: 0 }
    this.lastAppliedSequence.set(params.session_key, cursor.sequence)
    await this.applySync(params.session_key, result.sync)
    return result
  }

  async resume(params: StateResumeParams): Promise<StateSyncResultV1> {
    const result = await this.request<StateSyncResultV1>('state/resume', params)
    await this.applySync(params.session_key, result)
    return result
  }

  startTurn(params: TurnStartParams): Promise<TurnStartResultV1> {
    return this.request<TurnStartResultV1>('turn/start', params)
  }

  cancelTurn(params: TurnCancelParams): Promise<TurnCancelResultV1> {
    return this.request<TurnCancelResultV1>('turn/cancel', params)
  }

  close(): void {
    this.closedByCaller = true
    this.setStatus('closed')
    this.socket?.close(1000, 'client closed')
    this.socket = null
    this.rejectPending(new Error('Neuro-Link client closed'))
  }

  private request<T>(method: string, params: unknown): Promise<T> {
    if (!this.socket || this.status !== 'connected' && method !== 'protocol/hello') {
      throw new Error('Neuro-Link socket is not connected')
    }
    const id = `nl-${++this.requestSequence}`
    const payload = JSON.stringify({ jsonrpc: JSON_RPC_VERSION, id, method, params })
    return new Promise<T>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.pending.delete(id)
        reject(new Error(`Neuro-Link request timed out: ${method}`))
      }, this.options.requestTimeoutMs)
      this.pending.set(id, { resolve: resolve as (value: unknown) => void, reject, timer })
      try {
        this.socket?.send(payload)
      } catch (error) {
        clearTimeout(timer)
        this.pending.delete(id)
        reject(error)
      }
    })
  }

  private attachSocket(socket: WebSocketLike): void {
    socket.addEventListener('message', (event) => this.handleMessage(event.data))
    socket.addEventListener('error', () => {
      if (this.status !== 'closed') this.setStatus('error')
    })
    socket.addEventListener('close', () => {
      this.socket = null
      this.rejectPending(new Error('Neuro-Link socket disconnected before the response was received'))
      if (!this.closedByCaller) this.setStatus('error')
    })
  }

  private waitForOpen(socket: WebSocketLike): Promise<void> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Neuro-Link socket open timed out')), this.options.requestTimeoutMs)
      socket.addEventListener('open', () => {
        clearTimeout(timer)
        resolve()
      })
      socket.addEventListener('error', () => {
        clearTimeout(timer)
        reject(new Error('Neuro-Link socket failed to open'))
      })
      socket.addEventListener('close', () => {
        clearTimeout(timer)
        reject(new Error('Neuro-Link socket closed while opening'))
      })
    })
  }

  private handleMessage(raw: unknown): void {
    if (typeof raw !== 'string') {
      this.failConnection(new Error('Neuro-Link only accepts text JSON frames'))
      return
    }
    let value: Record<string, unknown>
    try {
      value = JSON.parse(raw) as Record<string, unknown>
    } catch {
      this.failConnection(new Error('Neuro-Link received malformed JSON'))
      return
    }
    const id = value.id as RpcId | undefined
    if (id !== undefined && id !== null) {
      const pending = this.pending.get(String(id))
      if (pending) {
        this.pending.delete(String(id))
        clearTimeout(pending.timer)
        if (value.error && typeof value.error === 'object') {
          pending.reject(new NeuroLinkProtocolError(value.error as RpcErrorV1['error']))
        } else {
          pending.resolve((value as unknown as RpcResponseV1<unknown>).result)
        }
        return
      }
    }
    if (typeof value.method !== 'string' || !value.params || typeof value.params !== 'object') return
    const params = value.params as EnvelopeNotificationParams
    if (!params.envelope || typeof params.envelope !== 'object') return
    const envelope = params.envelope
    const sequence = envelope.correlation.sequence
    if (!Number.isInteger(sequence) || sequence === undefined || sequence < 0) {
      this.failConnection(new Error('Neuro-Link projection is missing a valid cursor sequence'))
      return
    }
    const event: ProjectionEnvelopeEvent = {
      method: value.method,
      cursor: { stream: envelope.correlation.session_key, sequence },
      envelope,
      source: 'live',
    }
    this.enqueueProjection(event)
  }

  private async applySync(sessionKey: string, sync: StateSyncResultV1): Promise<void> {
    const events = [...sync.events, ...sync.snapshot]
      .map((event) => ({ ...event, source: 'replay' as const }))
      .filter((event) => event.cursor.stream === sessionKey)
      .sort((left, right) => left.cursor.sequence - right.cursor.sequence)
    for (const event of events) {
      await this.applyProjection(event)
    }
    const applied = this.lastAppliedSequence.get(sessionKey) ?? 0
    if (sync.head.sequence > applied) this.lastAppliedSequence.set(sessionKey, sync.head.sequence)
  }

  private enqueueProjection(event: ProjectionEnvelopeEvent): void {
    this.projectionQueue = this.projectionQueue
      .then(() => this.applyProjection(event))
      .catch((error) => {
        this.failConnection(error instanceof Error ? error : new Error(String(error)))
      })
  }

  private async applyProjection(event: ProjectionEnvelopeEvent): Promise<void> {
    const previous = this.lastAppliedSequence.get(event.cursor.stream) ?? 0
    if (event.cursor.sequence <= previous) return
    if (event.method === 'state/gap') {
      // A live gap is explicitly non-authoritative. Rebuild from the last
      // applied cursor instead of persisting/ACKing the gap marker itself.
      const sync = await this.request<StateSyncResultV1>('state/resume', {
        session_key: event.cursor.stream,
        cursor: { stream: event.cursor.stream, sequence: previous },
      })
      await this.applySync(event.cursor.stream, sync)
      return
    }
    await this.config.onProjection?.(event)
    this.lastAppliedSequence.set(event.cursor.stream, event.cursor.sequence)
    const ack: EventAckParams = { session_key: event.cursor.stream, cursor: event.cursor }
    if (this.status === 'connected') {
      await this.request<EventAckResultV1>('event/ack', ack)
    }
  }

  private failConnection(error: Error): void {
    this.setStatus('error')
    this.rejectPending(error)
    this.socket?.close(1002, error.message)
  }

  private rejectPending(error: Error): void {
    for (const [id, pending] of this.pending) {
      clearTimeout(pending.timer)
      pending.reject(error)
      this.pending.delete(id)
    }
  }

  private setStatus(status: NeuroLinkConnectionStatus): void {
    this.status = status
    this.config.onStatusChange?.(status)
  }
}
