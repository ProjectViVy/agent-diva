/**
 * Thin-shell host seam (DN-5). The only frontend ↔ native boundary.
 *
 * - `vivyCall` forwards a VIVY RPC request verbatim; the shell never
 *   interprets business method names.
 * - `onVivyEvent` subscribes to the single `vivy:event` channel carrying
 *   VIVY domain events plus bridge status (gap / transport lost).
 *
 * Later stories (DN-1..DN-6) rewire retired legacy invoke consumers onto
 * this seam or delete them; nothing else may call `invoke` for VIVY traffic.
 */
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface VivyCallRequest {
  method: string
  params?: unknown
  timeoutMs?: number
}

export interface BridgeError {
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
  code: number
  message: string
  data?: unknown
}

export type WireEvent =
  | { kind: 'vivy'; method: string; params: unknown }
  | { kind: 'bridge'; status: 'gap' | 'lost' }

/** True when running inside the Tauri shell (vs. plain browser dev). */
export function isTauriShell(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

export async function vivyCall(request: VivyCallRequest): Promise<unknown> {
  return invoke<unknown>('vivy_call', { request })
}

export function onVivyEvent(handler: (event: WireEvent) => void): Promise<UnlistenFn> {
  return listen<WireEvent>('vivy:event', (e) => handler(e.payload))
}

// --- C2-4 native speech seam (DN-6B commands, DN-6C typed facade) ----------
// Only this file names literal command strings, Raw bodies, and header keys;
// everything upstream talks to the typed facade in src/api/speech.ts.

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
  return invoke<SpeechConfigReadback>('speech_config_get')
}

export function speechConfigUpdate(input: { base_revision: number; preferences: unknown }): Promise<SpeechConfigReadback> {
  return invoke<SpeechConfigReadback>('speech_config_update', input)
}

export function speechCredentialSet(provider: string, key: string): Promise<{ provider: string; present: boolean }> {
  return invoke('speech_credential_set', { provider, key })
}

export function speechCredentialDelete(provider: string): Promise<{ provider: string; present: boolean }> {
  return invoke('speech_credential_delete', { provider })
}

export function speechContextSet(sessionId: string, generation: number): Promise<SpeechContext> {
  return invoke<SpeechContext>('speech_context_set', { session_id: sessionId, generation })
}

export function speechTranscribe(wav: ArrayBuffer, identity: SpeechIdentity): Promise<TranscribeReply> {
  // Raw body + bounded metadata header per C2-4 — never provider auth.
  return invoke<TranscribeReply>('speech_transcribe', wav, {
    headers: {
      'x-diva-speech-meta': JSON.stringify({ identity, mime_type: 'audio/wav' }),
    },
  })
}

export function speechSynthesize(identity: SpeechIdentity, text: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>('speech_synthesize', { identity, text })
}

export function speechCancel(requestId: string): Promise<{ request_id: string; status: 'cancelled' | 'settled' }> {
  return invoke('speech_cancel', { request_id: requestId })
}

export interface VoiceAssetDescriptor {
  asset_id: string
  display_name: string
  mime_type: string
  size_bytes: number
  digest_sha256: string
}

export function voiceAssetImport(bytes: ArrayBuffer, displayName: string, mimeType: string): Promise<VoiceAssetDescriptor> {
  return invoke<VoiceAssetDescriptor>('voice_asset_import', bytes, {
    headers: {
      'x-diva-asset-meta': JSON.stringify({ display_name: displayName, mime_type: mimeType }),
    },
  })
}

export function voiceAssetList(): Promise<VoiceAssetDescriptor[]> {
  return invoke<VoiceAssetDescriptor[]>('voice_asset_list')
}

export function voiceAssetRead(assetId: string): Promise<ArrayBuffer> {
  return invoke<ArrayBuffer>('voice_asset_read', { asset_id: assetId })
}

export function voiceAssetDelete(assetId: string): Promise<'deleted' | 'pending'> {
  return invoke('voice_asset_delete', { asset_id: assetId })
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
  return listen<SpeechDiagnostic>('speech:diagnostic', (e) => handler(e.payload))
}
