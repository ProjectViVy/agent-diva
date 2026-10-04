/**
 * Thin-shell host seam (DN-5, DN-W3). The only frontend ↔ native boundary.
 *
 * - `vivyCall` forwards a VIVY RPC request verbatim through the bound
 *   `RuntimeService.VivyCall`; the shell never interprets business method
 *   names. `CallReply` carries the normalized `{ok,result,error}` envelope —
 *   host errors keep kind/code/message/data instead of being flattened.
 * - `onVivyEvent` subscribes to the single `vivy:event` channel carrying
 *   VIVY domain events plus bridge status (gap / transport lost).
 * - `nativeCall` is the generic dispatch channel for retained native
 *   commands (speech/credentials/voice assets/pet). Handlers land with W4;
 *   the Go side gates every privileged command on the native window
 *   identity (W3-3/W3-5).
 * - `mediaFetch` is the raw-binary route (W3-3): internal scheme fetch with
 *   the issued media token — bytes travel raw, never JSON/base64 expanded.
 */
import { Events } from '@wailsio/runtime'
// Generated Wails bindings (DN-W3 Task 2); regenerate with:
//   wails3 generate bindings -f '-tags gtk3 vivy_headless' \
//     -d agent-diva-gui/src/generated/wails ./cmd/diva
import { RuntimeService } from '../generated/wails/github.com/ProjectViVy/agent-diva/internal/desktop'

export interface VivyCallRequest {
  method: string
  params?: unknown
  timeoutMs?: number
}

export interface BridgeError {
  kind:
    | 'invalid_input'
    | 'not_ready'
    | 'incompatible_abi'
    | 'closed'
    | 'already_initialized'
    | 'transport_lost'
    | 'timeout'
    | 'cancelled'
    | 'event_gap'
    | 'internal'
    | 'load_failed'
    | 'rpc'
  code: number
  message: string
  data?: unknown
}

export type WireEvent =
  | { kind: 'vivy'; method: string; params: unknown }
  | { kind: 'bridge'; status: 'gap' | 'lost' }

export type UnlistenFn = () => void

/**
 * True inside the real Wails shell — the injected runtime prelude sets
 * `window._wails.environment`; importing @wailsio/runtime alone does not
 * (plain browser dev / tests stay browser-mode).
 */
export function isDesktopShell(): boolean {
  return typeof window !== 'undefined' &&
    (window as unknown as { _wails?: { environment?: unknown } })._wails?.environment != null
}

/** @deprecated Use isDesktopShell(); kept for deferred pet modules. */
export const isTauriShell = isDesktopShell

interface CallReplyEnvelope {
  ok: boolean
  result?: unknown
  error?: BridgeError
}

function normalizeCallReply(reply: CallReplyEnvelope): unknown {
  if (reply?.ok) return reply.result
  const err = reply?.error
  throw (err ?? { kind: 'internal', code: -32086, message: 'call failed without detail' }) satisfies BridgeError
}

export async function vivyCall(request: VivyCallRequest): Promise<unknown> {
  const reply = (await RuntimeService.VivyCall(request)) as CallReplyEnvelope
  return normalizeCallReply(reply)
}

export function onVivyEvent(handler: (event: WireEvent) => void): Promise<UnlistenFn> {
  const off = Events.On('vivy:event', (e: { data: WireEvent }) => handler(e.data))
  return Promise.resolve(() => off())
}

// --- Generic native command channel (retained commands; handlers land W4) --

export async function nativeCall<T = unknown>(command: string, payload?: unknown): Promise<T> {
  const reply = (await RuntimeService.DesktopDispatch({ command, payload })) as CallReplyEnvelope
  return normalizeCallReply(reply) as T
}

export function nativeListen<T>(channel: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  const off = Events.On(channel, (e: { data: T }) => handler(e.data))
  return Promise.resolve(() => off())
}

let mediaTokenPromise: Promise<string> | null = null

/** Issue (once) the media capability token — main-window-gated natively. */
export function mediaToken(): Promise<string> {
  mediaTokenPromise ??= RuntimeService.MediaToken() as Promise<string>
  return mediaTokenPromise
}

/**
 * Raw-binary fetch against the internal asset scheme. The token is attached
 * here; the window-id header is injected natively per request, so a forged
 * sender can never satisfy the capability.
 */
export async function mediaFetch(route: string, init: RequestInit = {}): Promise<Response> {
  const token = await mediaToken()
  const headers = new Headers(init.headers)
  headers.set('X-Diva-Media-Token', token)
  return fetch(route, { ...init, headers })
}

// --- C2-4 native speech seam (typed facade upstream: src/api/speech.ts) ----
// Command names stay identical to the retired Tauri shell; the Go
// DesktopDispatch routes them in W4.

export interface SpeechIdentity {
  request_id: string
  session_id: string
  run_id?: string
  utterance_id: string
  generation: number
}

export interface SpeechContext {
  session_id: string
  generation: number
}

export interface SpeechConfigReadback {
  schema: string
  revision: number
  preferences: unknown
  credential_state: { siliconflow: string; minimax: string }
  window_context: SpeechContext | null
}

export interface TranscribeReply {
  identity: SpeechIdentity
  status: 'transcribed' | 'no_speech'
  text: string
}

export function speechConfigGet(): Promise<SpeechConfigReadback> {
  return nativeCall('speech_config_get')
}

export function speechConfigUpdate(input: { base_revision: number; preferences: unknown }): Promise<SpeechConfigReadback> {
  return nativeCall('speech_config_update', input)
}

export function speechCredentialSet(provider: string, key: string): Promise<{ provider: string; present: boolean }> {
  return nativeCall('speech_credential_set', { provider, key })
}

export function speechCredentialDelete(provider: string): Promise<{ provider: string; present: boolean }> {
  return nativeCall('speech_credential_delete', { provider })
}

export function speechContextSet(sessionId: string, generation: number): Promise<SpeechContext> {
  return nativeCall('speech_context_set', { session_id: sessionId, generation })
}

export async function speechTranscribe(wav: ArrayBuffer, identity: SpeechIdentity): Promise<TranscribeReply> {
  // Raw body + bounded metadata header per C2-4 — never provider auth.
  const res = await mediaFetch('/media/speech/transcribe', {
    method: 'POST',
    headers: { 'x-diva-speech-meta': JSON.stringify({ identity, mime_type: 'audio/wav' }) },
    body: wav,
  })
  if (!res.ok) throw { kind: 'internal', code: res.status, message: `speech_transcribe: ${res.status}` } satisfies BridgeError
  return res.json()
}

export async function speechSynthesize(identity: SpeechIdentity, text: string): Promise<ArrayBuffer> {
  const res = await mediaFetch('/media/speech/synthesize', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ identity, text }),
  })
  if (!res.ok) throw { kind: 'internal', code: res.status, message: `speech_synthesize: ${res.status}` } satisfies BridgeError
  return res.arrayBuffer()
}

export function speechCancel(requestId: string): Promise<{ request_id: string; status: 'cancelled' | 'settled' }> {
  return nativeCall('speech_cancel', { request_id: requestId })
}

export interface VoiceAssetDescriptor {
  asset_id: string
  display_name: string
  mime_type: string
  size_bytes: number
  digest_sha256: string
}

export async function voiceAssetImport(bytes: ArrayBuffer, displayName: string, mimeType: string): Promise<VoiceAssetDescriptor> {
  const res = await mediaFetch('/media/voice-assets', {
    method: 'POST',
    headers: { 'x-diva-asset-meta': JSON.stringify({ display_name: displayName, mime_type: mimeType }) },
    body: bytes,
  })
  if (!res.ok) throw { kind: 'internal', code: res.status, message: `voice_asset_import: ${res.status}` } satisfies BridgeError
  return res.json()
}

export function voiceAssetList(): Promise<VoiceAssetDescriptor[]> {
  return nativeCall('voice_asset_list')
}

export async function voiceAssetRead(assetId: string): Promise<ArrayBuffer> {
  const res = await mediaFetch(`/media/voice-assets/${encodeURIComponent(assetId)}`)
  if (!res.ok) throw { kind: 'internal', code: res.status, message: `voice_asset_read: ${res.status}` } satisfies BridgeError
  return res.arrayBuffer()
}

export function voiceAssetDelete(assetId: string): Promise<'deleted' | 'pending'> {
  return nativeCall('voice_asset_delete', { asset_id: assetId })
}

export interface SpeechDiagnostic {
  phase: string
  kind: string
  provider?: string
  code?: string
  http_status?: number
  elapsed_ms: number
  bytes: number
  request_id: string
  utterance_id: string
  generation: number
  config_revision: number
}

export function onSpeechDiagnostic(handler: (diagnostic: SpeechDiagnostic) => void): Promise<UnlistenFn> {
  return nativeListen<SpeechDiagnostic>('speech:diagnostic', handler)
}
