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
