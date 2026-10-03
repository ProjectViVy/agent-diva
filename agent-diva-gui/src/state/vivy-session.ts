/**
 * Authoritative, replayable session/run projection (DN-1). Reduces VIVY
 * notifications and snapshot reads into per-run contiguous event streams.
 *
 * Ordering contract (P0-D1): dedup by (run_id, seq); the contiguous cursor
 * never advances across a missing sequence; on any gap (bridge `gap`
 * status or a seq jump) the run is marked `needsResync` and callers
 * re-read `run/log` + `approval/list` then replay — snapshots always win
 * over buffered events. Recovery is listener-before-snapshot: callers
 * subscribe via `attach` before issuing snapshot reads through the same
 * client.
 */
import type {
  Approval,
  ApprovalListResult,
  RunEvent,
  RunEventParams,
  RunGetResult,
  RunLogResult,
  SessionGetResult,
  SessionListResult,
  SessionMessage,
  VivySession,
  WireEvent,
} from '../api/vivy/contracts'
import type { VivyClient } from '../api/vivy/client'

export type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'gap' | 'lost'
export type RunPhase = 'active' | 'completed' | 'failed' | 'cancelled' | 'unknown'

export interface PendingInteraction {
  kind: 'approval' | 'question'
  id: string
  runId?: string
  toolCallId?: string
  toolName?: string
  args?: Record<string, unknown>
  expiresAt?: number
}

export interface RunProjection {
  runId: string
  sessionId?: string
  /** Last contiguous applied seq; seq <= cursor is a duplicate. */
  cursor: number
  /** Contiguous applied events in seq order. */
  events: RunEvent[]
  /** Out-of-order events waiting on a missing seq. */
  bufferedCount: number
  /** True when a seq jump or bridge gap was observed; replay required. */
  needsResync: boolean
  phase: RunPhase
  /** Concatenated `model.delta` text so far. */
  streamText: string
  /** Terminal `run.completed` summary (the terminal-only answer). */
  terminalSummary?: string
  pendingApprovals: Map<string, PendingInteraction>
  pendingQuestions: Map<string, PendingInteraction>
}

interface MutableRun extends RunProjection {
  buffered: Map<number, RunEvent>
}

const TERMINAL: Record<string, RunPhase> = {
  'run.completed': 'completed',
  'run.failed': 'failed',
  'run.cancelled': 'cancelled',
}

function newRun(runId: string): MutableRun {
  return {
    runId,
    cursor: 0,
    events: [],
    buffered: new Map(),
    bufferedCount: 0,
    needsResync: false,
    phase: 'active',
    streamText: '',
    pendingApprovals: new Map(),
    pendingQuestions: new Map(),
  }
}

export class VivySessionProjection {
  connection: ConnectionState = 'disconnected'
  readonly sessions = new Map<string, VivySession>()
  readonly messages = new Map<string, SessionMessage[]>()
  private readonly runs = new Map<string, MutableRun>()
  private readonly listeners = new Set<() => void>()
  private unsubscribe: (() => void) | null = null
  private client: VivyClient | null = null

  /** Notify on applied events, snapshots, and connection state changes. */
  subscribe(listener: () => void): () => void {
    this.listeners.add(listener)
    return () => this.listeners.delete(listener)
  }

  private emit(): void {
    for (const listener of this.listeners) listener()
  }

  /**
   * Register the event listener BEFORE any snapshot read. The returned
   * promise resolves once the subscription is live; callers then fetch
   * snapshots (session/list, run/log, approval/list) and feed them through
   * `applySnapshot` — events arriving in the gap are buffered by (run_id,
   * seq) and merge in order afterwards.
   */
  async attach(client: VivyClient): Promise<void> {
    this.client = client
    this.connection = 'connecting'
    this.unsubscribe = await client.onEvent((event) => this.reduce(event))
    if (this.connection === 'connecting') this.connection = 'connected'
  }

  detach(): void {
    this.unsubscribe?.()
    this.unsubscribe = null
    this.connection = 'disconnected'
  }

  run(runId: string): RunProjection | undefined {
    return this.runs.get(runId)
  }

  runIds(): string[] {
    return [...this.runs.keys()]
  }

  pendingInteractions(): PendingInteraction[] {
    const out: PendingInteraction[] = []
    for (const run of this.runs.values()) {
      out.push(...run.pendingApprovals.values(), ...run.pendingQuestions.values())
    }
    return out
  }

  /** Assistant-visible answer: streamed deltas, else the terminal summary. */
  answer(runId: string): string {
    const run = this.runs.get(runId)
    return run ? run.streamText || run.terminalSummary || '' : ''
  }

  reduce(event: WireEvent): void {
    if (event.kind === 'bridge') {
      this.reduceBridge(event.status)
      return
    }
    if (event.method === 'run/event') {
      const params = event.params as RunEventParams
      this.applyRunEvent(params.event)
      this.emit()
    }
    // Other notification methods are forwarded to domain reducers added by
    // later stories; unrecognized payloads are ignored, never fatal.
  }

  private reduceBridge(status: 'gap' | 'lost'): void {
    if (status === 'lost') {
      this.connection = 'lost'
    } else {
      this.connection = 'gap'
    }
    for (const run of this.runs.values()) run.needsResync = true
    this.emit()
  }

  private runFor(runId: string): MutableRun {
    let run = this.runs.get(runId)
    if (!run) {
      run = newRun(runId)
      this.runs.set(runId, run)
    }
    return run
  }

  private applyRunEvent(event: RunEvent): void {
    const run = this.runFor(event.run_id)
    if (event.seq <= run.cursor) return // late/duplicate — drop
    if (event.seq !== run.cursor + 1) {
      // Missing sequence: buffer, never advance the cursor across the gap.
      run.buffered.set(event.seq, event)
      run.bufferedCount = run.buffered.size
      run.needsResync = true
      return
    }
    run.buffered.set(event.seq, event)
    this.drain(run)
  }

  private drain(run: MutableRun): void {
    let next = run.cursor + 1
    while (run.buffered.has(next)) {
      const event = run.buffered.get(next) as RunEvent
      run.buffered.delete(next)
      this.apply(event, run)
      next = run.cursor + 1
    }
    run.bufferedCount = run.buffered.size
  }

  private apply(event: RunEvent, run: MutableRun): void {
    run.cursor = event.seq
    run.events.push(event)
    switch (event.type) {
      case 'model.delta': {
        const delta = event.payload.delta
        if (typeof delta === 'string') run.streamText += delta
        break
      }
      case 'tool.approval_required': {
        const p = event.payload
        if (typeof p.approval_id === 'string') {
          run.pendingApprovals.set(p.approval_id, {
            kind: 'approval',
            id: p.approval_id,
            runId: event.run_id,
            toolCallId: p.tool_call_id as string | undefined,
            toolName: p.tool_name as string | undefined,
            args: p.args as Record<string, unknown> | undefined,
            expiresAt: p.expires_at as number | undefined,
          })
        }
        break
      }
      case 'tool.approval_decided':
      case 'tool.approval_expired':
      case 'tool.approval_cancelled': {
        const p = event.payload
        if (typeof p.approval_id === 'string') run.pendingApprovals.delete(p.approval_id)
        break
      }
      case 'user.question_required': {
        const p = event.payload
        const id = (p.question_id ?? p.id) as string | undefined
        if (typeof id === 'string') {
          run.pendingQuestions.set(id, {
            kind: 'question',
            id,
            runId: event.run_id,
            toolCallId: p.tool_call_id as string | undefined,
            expiresAt: p.expires_at as number | undefined,
            args: typeof p.prompt === 'string' ? { prompt: p.prompt } : undefined,
          })
        }
        break
      }
      case 'user.question_answered':
      case 'user.question_cancelled':
      case 'user.question_expired': {
        const p = event.payload
        const id = (p.question_id ?? p.id) as string | undefined
        if (typeof id === 'string') run.pendingQuestions.delete(id)
        break
      }
      default: {
        const phase = TERMINAL[event.type]
        if (phase) {
          run.phase = phase
          if (event.type === 'run.completed' && typeof event.payload.summary === 'string') {
            run.terminalSummary = event.payload.summary
          }
        }
      }
    }
  }

  /** Apply an authoritative snapshot; snapshots always win. */
  applySnapshot(
    kind:
      | { type: 'session/list'; result: SessionListResult }
      | { type: 'session/get'; result: SessionGetResult }
      | { type: 'run/log'; result: RunLogResult }
      | { type: 'run/get'; result: RunGetResult }
      | { type: 'approval/list'; result: ApprovalListResult },
  ): void {
    switch (kind.type) {
      case 'session/list':
        for (const s of kind.result.sessions) this.sessions.set(s.id, s)
        break
      case 'session/get':
        this.sessions.set(kind.result.session.id, kind.result.session)
        this.messages.set(kind.result.session.id, kind.result.messages)
        break
      case 'run/log':
        this.applyRunLog(kind.result.events)
        break
      case 'run/get': {
        const run = this.runFor(kind.result.id)
        run.sessionId = kind.result.session_id
        if (kind.result.status !== 'active') {
          run.phase = (TERMINAL[kind.result.status] ?? kind.result.status) as RunPhase
        }
        break
      }
      case 'approval/list':
        this.applyApprovalList(kind.result.approvals)
        break
    }
    this.emit()
  }

  private applyRunLog(events: RunEvent[]): void {
    const byRun = new Map<string, RunEvent[]>()
    for (const e of events) {
      const list = byRun.get(e.run_id) ?? []
      list.push(e)
      byRun.set(e.run_id, list)
    }
    for (const [runId, list] of byRun) {
      const run = this.runFor(runId)
      list.sort((a, b) => a.seq - b.seq)
      for (const e of list) {
        if (e.seq <= run.cursor) continue
        run.buffered.set(e.seq, e)
      }
      this.drain(run)
      run.bufferedCount = run.buffered.size
      // A hole left after an authoritative replay means the backend never
      // emitted those seqs — the run stays flagged.
      run.needsResync = run.buffered.size > 0
    }
  }

  private applyApprovalList(approvals: Approval[]): void {
    // Snapshot is authoritative for pending interactions: anything decided
    // elsewhere (second window, restart) disappears; missed
    // approval_required events are reconstructed from `decision:'pending'`.
    const pending = new Map<string, Approval>()
    for (const a of approvals) if (a.decision === 'pending') pending.set(a.id, a)
    for (const run of this.runs.values()) run.pendingApprovals.clear()
    for (const a of pending.values()) {
      const run = this.runFor(a.run_id)
      run.pendingApprovals.set(a.id, {
        kind: 'approval',
        id: a.id,
        runId: a.run_id,
        toolCallId: a.tool_call_id,
        expiresAt: a.expires_at,
      })
    }
  }

  /**
   * Recovery after a bridge `gap`/`lost` or window reopen: re-read
   * authoritative state for every run that needs resync plus the pending
   * interaction list. Listeners were attached before this call.
   */
  async resync(): Promise<void> {
    if (!this.client) throw new Error('projection not attached')
    const client = this.client
    const approvalList = await client.approvalList()
    this.applySnapshot({ type: 'approval/list', result: approvalList })
    for (const run of this.runs.values()) {
      if (!run.needsResync) continue
      const log = await client.runLog(run.runId)
      this.applySnapshot({ type: 'run/log', result: log })
      const get = await client.runGet(run.runId)
      this.applySnapshot({ type: 'run/get', result: get })
    }
    if (this.connection === 'gap') this.connection = 'connected'
    this.emit()
  }
}
