import { onUnmounted, ref, shallowRef, type Ref } from 'vue'
import type {
  SessionOpenResultV1,
  TurnCancelParams,
  TurnCancelResultV1,
  TurnStartParams,
  TurnStartResultV1,
} from '../../protocol/neuro-link-v1'
import {
  cursorStorageKey,
  makeFrontendInstanceId,
  NeuroLinkClient,
  persistCursor,
  readPersistedCursor,
  tauriGatewayEndpoint,
  type GatewayEndpointProvider,
  type NeuroLinkClientOptions,
  type NeuroLinkConnectionStatus,
  type ProjectionEnvelopeEvent,
  type WebSocketFactory,
} from './client'
import {
  createNeuroLinkProjectionState,
  reduceNeuroLinkProjection,
  type NeuroLinkProjectionState,
} from './projection'

export interface NeuroLinkSessionOptions {
  workspaceRoot: Ref<string> | (() => string) | string
  storage?: Storage
  frontendInstanceId?: string
  endpoint?: GatewayEndpointProvider
  webSocketFactory?: WebSocketFactory
  capabilities?: string[]
  requestTimeoutMs?: number
  onProjection?: (event: ProjectionEnvelopeEvent, state: NeuroLinkProjectionState) => void | Promise<void>
  onStatusChange?: (status: NeuroLinkConnectionStatus) => void
  reconnect?: boolean
}

const BASE_RECONNECT_MS = 250
const MAX_RECONNECT_MS = 5_000

export function useNeuroLinkSession(options: NeuroLinkSessionOptions) {
  const connectionStatus = ref<NeuroLinkConnectionStatus>('idle')
  const currentSessionKey = ref<string | null>(null)
  const lastError = ref<string | null>(null)
  const projectionState = shallowRef<NeuroLinkProjectionState>(createNeuroLinkProjectionState())
  const client = shallowRef<NeuroLinkClient | null>(null)
  const generation = ref(0)
  let reconnectTimer: ReturnType<typeof setTimeout> | undefined
  let reconnectAttempt = 0
  let closed = false

  const storage = options.storage ?? (typeof localStorage === 'undefined' ? undefined : localStorage)
  const endpoint = options.endpoint ?? tauriGatewayEndpoint
  const frontendInstanceId = options.frontendInstanceId ?? makeFrontendInstanceId(storage)

  function workspaceRoot(): string {
    if (typeof options.workspaceRoot === 'string') return options.workspaceRoot
    if (typeof options.workspaceRoot === 'function') return options.workspaceRoot()
    return options.workspaceRoot.value
  }

  async function open(sessionKey: string): Promise<SessionOpenResultV1 | null> {
    closed = false
    generation.value += 1
    const localGeneration = generation.value
    clearReconnectTimer()
    client.value?.close()
    currentSessionKey.value = sessionKey
    projectionState.value = createNeuroLinkProjectionState(sessionKey)
    lastError.value = null
    reconnectAttempt = 0
    return connectAndOpen(sessionKey, localGeneration)
  }

  async function connectAndOpen(sessionKey: string, localGeneration: number): Promise<SessionOpenResultV1 | null> {
    if (closed || localGeneration !== generation.value) return null
    const root = workspaceRoot()
    const clientOptions: NeuroLinkClientOptions = {
      endpoint,
      frontendInstanceId,
      capabilities: options.capabilities,
      webSocketFactory: options.webSocketFactory,
      requestTimeoutMs: options.requestTimeoutMs,
      onStatusChange: (status) => {
        connectionStatus.value = status
        options.onStatusChange?.(status)
        if (status === 'error' && !closed && localGeneration === generation.value && options.reconnect !== false) {
          scheduleReconnect(sessionKey, localGeneration)
        }
      },
      onProjection: async (event) => {
        projectionState.value = reduceNeuroLinkProjection(projectionState.value, event)
        persistCursor(storage, root, event.cursor)
        await options.onProjection?.(event, projectionState.value)
      },
    }
    const nextClient = new NeuroLinkClient(clientOptions)
    client.value = nextClient
    try {
      await nextClient.connect()
      const cursor = readPersistedCursor(storage, root, sessionKey)
      const result = await nextClient.openSession({ session_key: sessionKey, durable_cursor: cursor })
      reconnectAttempt = 0
      return result
    } catch (error) {
      if (localGeneration !== generation.value || closed) return null
      lastError.value = error instanceof Error ? error.message : String(error)
      connectionStatus.value = 'error'
      scheduleReconnect(sessionKey, localGeneration)
      return null
    }
  }

  function scheduleReconnect(sessionKey: string, localGeneration: number): void {
    if (reconnectTimer || closed || localGeneration !== generation.value) return
    const delay = Math.min(MAX_RECONNECT_MS, BASE_RECONNECT_MS * 2 ** reconnectAttempt)
    reconnectAttempt += 1
    reconnectTimer = setTimeout(() => {
      reconnectTimer = undefined
      void connectAndOpen(sessionKey, localGeneration)
    }, delay)
  }

  function clearReconnectTimer(): void {
    if (reconnectTimer) clearTimeout(reconnectTimer)
    reconnectTimer = undefined
  }

  async function startTurn(params: TurnStartParams): Promise<TurnStartResultV1> {
    if (!client.value || connectionStatus.value !== 'connected') {
      throw new Error('Neuro-Link is not connected; turn outcome is not known')
    }
    return client.value.startTurn(params)
  }

  async function cancelTurn(params: TurnCancelParams): Promise<TurnCancelResultV1> {
    if (!client.value || connectionStatus.value !== 'connected') {
      throw new Error('Neuro-Link is not connected; cancellation outcome is not known')
    }
    return client.value.cancelTurn(params)
  }

  function close(): void {
    closed = true
    generation.value += 1
    clearReconnectTimer()
    client.value?.close()
    client.value = null
    connectionStatus.value = 'closed'
  }

  onUnmounted(close)

  return {
    connectionStatus,
    currentSessionKey,
    lastError,
    projectionState,
    frontendInstanceId,
    open,
    startTurn,
    cancelTurn,
    close,
    cursorKey: (sessionKey: string) => cursorStorageKey(workspaceRoot(), sessionKey),
  }
}
