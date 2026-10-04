/**
 * OBS-06 host-connection + usage projection. The connection state is
 * derived from the shell (preview/native), the `initialize` handshake
 * and transport bridge events — never a fabricated health RPC. Usage
 * refreshes are coalesced to one in-flight snapshot and fenced by a
 * generation counter, so a delayed response after a param switch or a
 * reconnect can never overwrite the current totals.
 */
import type { WireEvent } from '../api/vivy/contracts'
import type { VivyUsageCoverage, VivyTokenUsageSnapshot } from '../api/vivy/contracts'
import type { VivyClient } from '../api/vivy/client'
import { getTokenUsage, type TokenUsageParams } from '../api/vivy/observability'
import { isTauriShell } from '../platform/desktop-host'

export type HostConnectionState =
  | 'preview'
  | 'connecting'
  | 'connected'
  | 'gap'
  | 'lost'
  | 'unavailable'
  | 'disconnected'

export interface HostConnection {
  state: HostConnectionState
  /** Present only when the initialize handshake actually succeeded. */
  protocolVersion?: string
  capabilities: string[]
}

export type UsageCoverageState = 'empty' | 'legacy' | 'partial' | 'complete'

export interface UsageView {
  state: UsageCoverageState
  coverage: VivyUsageCoverage
  /** `request_count` semantics: usage reports (reported + legacy rows),
   * never billed calls. */
  reportedReports: number
  unknownBuckets: string[]
}

/** Maps the v2 snapshot onto the four honest coverage labels. */
export function usageView(snap: VivyTokenUsageSnapshot): UsageView {
  const state = snap.coverage.state
  return {
    state:
      state === 'complete' || state === 'partial' || state === 'legacy' ? state : 'empty',
    coverage: snap.coverage,
    reportedReports: snap.total.request_count,
    unknownBuckets: snap.coverage.unknown_buckets ?? [],
  }
}

export type CostDisplay = { kind: 'known'; usd: number } | { kind: 'unknown' }

/** cost_known:false is a zero placeholder — unknown, never "free". */
export function costDisplay(usd: number, costKnown: boolean): CostDisplay {
  return costKnown ? { kind: 'known', usd } : { kind: 'unknown' }
}

export class VivyObservabilityController {
  private conn: HostConnection = { state: 'disconnected', capabilities: [] }
  private snap: VivyTokenUsageSnapshot | null = null
  private lastError: string | null = null
  private staleFlag = false
  private generation = 0
  private inFlight: { key: string; gen: number; promise: Promise<VivyTokenUsageSnapshot | null> } | null =
    null
  private client: VivyClient | null = null
  private unsubscribe: (() => void) | null = null
  private readonly listeners = new Set<() => void>()

  constructor(private readonly isNative: () => boolean) {}

  subscribe(listener: () => void): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  private emit(): void {
    for (const l of this.listeners) l()
  }

  connection(): HostConnection {
    return this.conn
  }

  snapshot(): VivyTokenUsageSnapshot | null {
    return this.snap
  }

  error(): string | null {
    return this.lastError
  }

  /** True while retained totals may lag the live host: refresh failed,
   * a run finished since the snapshot, or the link degraded. */
  stale(): boolean {
    return this.staleFlag
  }

  async attach(client: VivyClient): Promise<void> {
    this.generation++
    if (!this.isNative()) {
      this.conn = { state: 'preview', capabilities: [] }
      this.emit()
      return
    }
    this.client = client
    this.conn = { state: 'connecting', capabilities: [] }
    this.emit()
    this.unsubscribe = await client.onEvent((event) => this.reduce(event))
    try {
      const init = await client.initialize()
      this.conn = {
        state: 'connected',
        protocolVersion: String(init.protocol_version),
        capabilities: init.capabilities ?? [],
      }
    } catch {
      // ABI/host mismatch is explicit: unavailable, never "connected".
      this.conn = { state: 'unavailable', capabilities: [] }
    }
    this.emit()
  }

  detach(): void {
    this.generation++
    this.unsubscribe?.()
    this.unsubscribe = null
    this.client = null
    this.conn = { state: 'disconnected', capabilities: [] }
    this.emit()
  }

  private reduce(event: WireEvent): void {
    if (event.kind === 'bridge') {
      // Reconnect invalidates any in-flight snapshot response.
      this.generation++
      this.conn = { ...this.conn, state: event.status === 'lost' ? 'lost' : 'gap' }
      this.staleFlag = this.snap !== null
      this.emit()
      return
    }
    if (event.kind === 'vivy' && event.method === 'run/event' && this.snap) {
      this.staleFlag = true
      this.emit()
    }
  }

  /**
   * Coalesced refresh: identical in-flight params share one request; a
   * newer request, detach, or bridge reconnect fences the older
   * response so it resolves null and cannot replace current totals.
   * Rejection keeps prior totals marked stale — never a fabricated zero.
   */
  refresh(params: TokenUsageParams): Promise<VivyTokenUsageSnapshot | null> {
    const key = `${params.period ?? ''}|${params.session_limit ?? ''}`
    if (this.inFlight && this.inFlight.key === key && this.inFlight.gen === this.generation) {
      return this.inFlight.promise
    }
    const client = this.client
    const gen = ++this.generation
    if (!client || (this.conn.state !== 'connected' && this.conn.state !== 'gap')) {
      this.lastError = 'host connection unavailable'
      this.staleFlag = this.snap !== null
      this.emit()
      return Promise.resolve(null)
    }
    const promise = getTokenUsage(client, params)
      .then((result) => {
        if (gen !== this.generation) return null
        this.snap = result
        this.lastError = null
        this.staleFlag = false
        this.emit()
        return result
      })
      .catch((e) => {
        if (gen !== this.generation) return null
        this.lastError = e instanceof Error ? e.message : String(e)
        this.staleFlag = this.snap !== null
        this.emit()
        return null
      })
      .finally(() => {
        if (this.inFlight?.promise === promise) this.inFlight = null
      })
    this.inFlight = { key, gen, promise }
    return promise
  }
}

/** Shared console observability lane bound to the embedded client. */
export const vivyObservability = new VivyObservabilityController(isTauriShell)
