/**
 * OBS-07 read-only trajectory projection. The `trajectory/session`
 * snapshot is authoritative — it replaces state wholesale with stable
 * backend IDs and per-run watermarks. Live events only advance per-run
 * seq cursors, update run activity from the durable vocabulary and mark
 * the retained rows stale: they never fabricate record/request rows,
 * and a gap marks the run visibly incomplete until the next snapshot.
 */
import type {
  RunEvent,
  TrajectoryRunActivity,
  TrajectorySession,
} from '../api/vivy/contracts'

export interface TrajectoryView {
  sessionId: string
  projectionVersion: number
  turns: number
  records: TrajectorySession['records']
  requests: TrajectorySession['requests']
  activity: TrajectoryRunActivity[]
  hasOlderRuns: boolean
  /** Runs whose folded cursor fell behind — visible incompleteness. */
  incompleteRuns: string[]
  /** Live events landed after the last snapshot; rows may lag. */
  stale: boolean
}

const TERMINAL_ACTIVITY: Record<string, string> = {
  'run.completed': 'completed',
  'run.failed': 'failed',
  'run.cancelled': 'cancelled',
}

export class TrajectoryProjection {
  private snap: TrajectorySession | null = null
  private cursors = new Map<string, number>()
  private incomplete = new Set<string>()
  private staleRows = false
  private readonly overrides = new Map<string, { state: string; waitKind?: string }>()

  /** Snapshot wins: replaces rows, cursors and incompleteness wholesale. */
  load(snapshot: TrajectorySession): void {
    this.snap = snapshot
    this.cursors = new Map(Object.entries(snapshot.watermarks))
    this.incomplete.clear()
    this.overrides.clear()
    this.staleRows = false
  }

  clear(): void {
    this.snap = null
    this.cursors.clear()
    this.incomplete.clear()
    this.overrides.clear()
    this.staleRows = false
  }

  /** Every run's cursor is suspect after a bridge gap. */
  markGap(): void {
    for (const runId of this.cursors.keys()) this.incomplete.add(runId)
  }

  /** Per-run contiguous cursor — dedup/resume bound for subscribers. */
  cursor(runId: string): number {
    return this.cursors.get(runId) ?? 0
  }

  needsResync(runId: string): boolean {
    return this.incomplete.has(runId)
  }

  /**
   * Reduce one live journal event. Returns true when the event was
   * new/relevant. Duplicate or late seqs are dropped; a seq jump marks
   * the run incomplete and never advances the cursor across the gap.
   */
  applyRunEvent(event: RunEvent): boolean {
    if (!this.snap) return false
    const runId = event.run_id
    const cursor = this.cursors.get(runId) ?? 0
    if (event.seq <= cursor) return false
    if (event.seq !== cursor + 1) {
      this.incomplete.add(runId)
      this.staleRows = true
      return true
    }
    this.cursors.set(runId, event.seq)
    this.foldActivity(event)
    this.staleRows = true
    return true
  }

  private foldActivity(event: RunEvent): void {
    const terminal = TERMINAL_ACTIVITY[event.type]
    if (terminal) {
      this.overrides.set(event.run_id, { state: terminal })
      return
    }
    switch (event.type) {
      case 'tool.approval_required':
        this.overrides.set(event.run_id, { state: 'waiting', waitKind: 'approval' })
        break
      case 'user.question_required':
        this.overrides.set(event.run_id, { state: 'waiting', waitKind: 'question' })
        break
      case 'tool.approval_decided':
      case 'tool.approval_expired':
      case 'tool.approval_cancelled':
      case 'user.question_answered':
      case 'user.question_cancelled':
      case 'user.question_expired':
        this.overrides.set(event.run_id, { state: 'active' })
        break
      case 'model.request':
        this.overrides.set(event.run_id, { state: 'active' })
        break
      default:
        break
    }
  }

  view(): TrajectoryView | null {
    const snap = this.snap
    if (!snap) return null
    const activity = snap.run_activity.map((a) => {
      const o = this.overrides.get(a.run_id)
      if (!o) return a
      const next: TrajectoryRunActivity = { ...a, activity_state: o.state }
      if (o.waitKind) next.wait_kind = o.waitKind
      else delete next.wait_kind
      return next
    })
    return {
      sessionId: snap.session_id,
      projectionVersion: snap.projection_version,
      turns: snap.turns,
      records: snap.records,
      requests: snap.requests,
      activity,
      hasOlderRuns: snap.has_older_runs,
      incompleteRuns: [...this.incomplete],
      stale: this.staleRows,
    }
  }
}
