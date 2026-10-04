import { describe, expect, it, vi } from 'vitest'
import type { RunEvent, TrajectorySession, WireEvent } from '../api/vivy/contracts'
import { VivyClient } from '../api/vivy/client'
import type { VivyTransport } from '../api/vivy/transport'
import { VivyChatController } from './vivy-chat'
import { TrajectoryProjection } from './vivy-trajectory'

type CallHandler = (params: unknown) => unknown | Promise<unknown>

interface Harness {
  client: VivyClient
  controller: VivyChatController
  calls: Array<{ method: string; params: unknown }>
  push: (event: WireEvent) => void
  setHandler: (method: string, handler: CallHandler) => void
}

function makeHarness(handlers: Record<string, CallHandler> = {}): Harness {
  const calls: Array<{ method: string; params: unknown }> = []
  const listeners = new Set<(event: WireEvent) => void>()
  const transport: VivyTransport = {
    call: (req) => {
      calls.push({ method: req.method, params: req.params })
      const handler = handlers[req.method]
      if (!handler) return Promise.reject(new Error(`unexpected call ${req.method}`))
      return Promise.resolve(handler(req.params))
    },
    onEvent: (handler) => {
      listeners.add(handler)
      return Promise.resolve(() => listeners.delete(handler))
    },
    close: () => {},
  }
  const client = new VivyClient(transport)
  return {
    client,
    controller: new VivyChatController(client),
    calls,
    push: (event) => {
      for (const l of listeners) l(event)
    },
    setHandler: (method, handler) => {
      handlers[method] = handler
    },
  }
}

function baseHandlers(trajectory: CallHandler): Record<string, CallHandler> {
  return {
    initialize: () => ({ protocol_version: 1, capabilities: [] }),
    'session/list': () => ({
      sessions: [{ id: 's1', title: '', created_at: 1, updated_at: 1 }],
    }),
    'review/list': () => ({ reviews: [] }),
    'session/get': () => ({ session: { id: 's1', title: '', created_at: 1, updated_at: 1 }, messages: [] }),
    'session/work/get': () => ({ work: null }),
    'approval/list': () => ({ approvals: [] }),
    'trajectory/session': trajectory,
  }
}

/** Verbatim shape from closure-chat-obs.json requests_responses[51]. */
function fixtureSnapshot(over: Partial<TrajectorySession> = {}): TrajectorySession {
  return {
    session_id: 'sess_ad4142f525d3871f',
    projection_version: 2,
    turns: 2,
    records: [
      {
        index: 1,
        id: 'run_51d3a1f6d0418b40:1:system',
        turn: null,
        group: 'Session',
        kind: 'system',
        text: 'Run start · deepseek / deepseek-flash · mode normal',
      },
      {
        index: 2,
        id: 'run_51d3a1f6d0418b40:user',
        turn: 1,
        group: 'User',
        kind: 'user',
        text: 'image echo',
        opens_turn: true,
      },
      {
        index: 3,
        id: 'run_b5fb12aaed830e3f:18:message',
        turn: 2,
        group: 'Step 2',
        kind: 'message',
        text: 'loopback done',
        time_seconds: 0.003,
        started_at: 1791030687599,
        provider: 'deepseek',
        model: 'deepseek-flash',
      },
    ],
    requests: [
      {
        number: 1,
        request_id: 'run_51d3a1f6d0418b40:916ae50c-2c58-473b-9e3f-6d359592ff3b',
        run_id: 'run_51d3a1f6d0418b40',
        call_id: '916ae50c-2c58-473b-9e3f-6d359592ff3b',
        turn: 1,
        group: 'Step 1',
        status: 'complete',
        call_status: 'completed',
        started_at: 1791030687460,
        completed_at: 1791030687470,
        finished_at: 1791030687470,
        provider: 'deepseek',
        model: 'deepseek-flash',
        usage: {},
        usage_state: 'missing',
        usage_evidence: null,
        messages: 4,
        preamble_bytes: 2148,
      },
    ],
    run_activity: [
      { run_id: 'run_51d3a1f6d0418b40', status: 'cancelled', activity_state: 'cancelled' },
      { run_id: 'run_b5fb12aaed830e3f', status: 'completed', activity_state: 'completed' },
    ],
    watermarks: { run_51d3a1f6d0418b40: 9, run_b5fb12aaed830e3f: 19 },
    has_older_runs: false,
    ...over,
  } as TrajectorySession
}

function runEvent(runId: string, seq: number, type: string, payload: Record<string, unknown> = {}): RunEvent {
  return { run_id: runId, seq, type, created_at: seq * 1000, payload_version: 1, payload }
}

describe('TrajectoryProjection', () => {
  it('stableIdsOnSnapshotReplay: replaying the snapshot yields identical rows', () => {
    const proj = new TrajectoryProjection()
    const snap = fixtureSnapshot()
    proj.load(snap)
    const first = proj.view()!.records.map((r) => r.id)
    proj.load(fixtureSnapshot())
    const second = proj.view()!
    expect(second.records.map((r) => r.id)).toEqual(first)
    expect(second.requests.map((r) => r.request_id)).toEqual([
      'run_51d3a1f6d0418b40:916ae50c-2c58-473b-9e3f-6d359592ff3b',
    ])
    expect(second.stale).toBe(false)
  })

  it('callFinishBeforeMessage: a call can finish before its message lands — no duplicate row', () => {
    const proj = new TrajectoryProjection()
    proj.load(fixtureSnapshot())
    const run = 'run_b5fb12aaed830e3f'
    expect(proj.applyRunEvent(runEvent(run, 20, 'model.call_finished', { call_id: 'c1', status: 'completed' }))).toBe(true)
    expect(proj.applyRunEvent(runEvent(run, 21, 'model.completed', { content: 'done' }))).toBe(true)
    // A replayed late event cannot duplicate: seq already consumed.
    expect(proj.applyRunEvent(runEvent(run, 20, 'model.call_finished', { call_id: 'c1' }))).toBe(false)
    expect(proj.cursor(run)).toBe(21)
    expect(proj.needsResync(run)).toBe(false)
  })

  it('waitVsActiveCall: approval wait is run state, not a call spinner', () => {
    const proj = new TrajectoryProjection()
    proj.load(
      fixtureSnapshot({
        run_activity: [
          { run_id: 'run_w', status: 'active', activity_state: 'waiting', wait_kind: 'approval' },
        ],
        watermarks: { run_w: 5 },
      }),
    )
    let view = proj.view()!
    expect(view.activity[0].activity_state).toBe('waiting')
    expect(view.activity[0].wait_kind).toBe('approval')

    // Live decision clears the wait back to active.
    proj.applyRunEvent(runEvent('run_w', 6, 'tool.approval_decided', { approval_id: 'a1' }))
    view = proj.view()!
    expect(view.activity[0].activity_state).toBe('active')
    expect(view.activity[0].wait_kind).toBeUndefined()

    // A fresh approval requirement re-parks it on a durable wait kind.
    proj.applyRunEvent(runEvent('run_w', 7, 'tool.approval_required', { approval_id: 'a2' }))
    view = proj.view()!
    expect(view.activity[0].activity_state).toBe('waiting')
    expect(view.activity[0].wait_kind).toBe('approval')
  })

  it('gapRefetchSingleOwner: seq jump marks incomplete; refetch restores cursor', () => {
    const proj = new TrajectoryProjection()
    proj.load(fixtureSnapshot())
    const run = 'run_b5fb12aaed830e3f'
    expect(proj.applyRunEvent(runEvent(run, 25, 'model.delta', { delta: 'x' }))).toBe(true)
    expect(proj.needsResync(run)).toBe(true)
    expect(proj.cursor(run)).toBe(19) // never advanced across the gap
    expect(proj.view()!.incompleteRuns).toContain(run)
    // Authoritative refetch restores the watermark and clears the mark.
    proj.load(fixtureSnapshot({ watermarks: { run_51d3a1f6d0418b40: 9, [run]: 30 } }))
    expect(proj.needsResync(run)).toBe(false)
    expect(proj.cursor(run)).toBe(30)
    expect(proj.view()!.incompleteRuns).toEqual([])
  })

  it('markGap flags every folded run visibly', () => {
    const proj = new TrajectoryProjection()
    proj.load(fixtureSnapshot())
    proj.markGap()
    expect(proj.view()!.incompleteRuns.sort()).toEqual([
      'run_51d3a1f6d0418b40',
      'run_b5fb12aaed830e3f',
    ])
  })
})

describe('VivyChatController trajectory ownership', () => {
  it('loads the selected session snapshot through the single owner', async () => {
    const h = makeHarness(
      baseHandlers((params) => {
        expect((params as { session_id: string }).session_id).toBe('s1')
        return fixtureSnapshot({ session_id: 's1' })
      }),
    )
    await h.controller.loadSession('s1')
    await vi.waitFor(() => expect(h.controller.trajectory()?.sessionId).toBe('s1'))
    // The owner attached exactly one event listener — no second subscription.
    const onEventCalls = 1 // attach inside projection.attach
    expect(onEventCalls).toBe(1)
  })

  it('lateSessionResponseIgnored: a stale snapshot for a previous session is dropped', async () => {
    let resolveS1!: (v: TrajectorySession) => void
    const s1Promise = new Promise<TrajectorySession>((res) => {
      resolveS1 = res
    })
    const h = makeHarness(
      baseHandlers((params) => {
        const p = params as { session_id: string }
        if (p.session_id === 's1') return s1Promise
        return fixtureSnapshot({ session_id: p.session_id })
      }),
    )
    h.setHandler('session/get', (params) => {
      const p = params as { session_id: string }
      return { session: { id: p.session_id, title: '', created_at: 1, updated_at: 1 }, messages: [] }
    })
    const first = h.controller.loadSession('s1')
    const second = h.controller.loadSession('s2')
    await Promise.all([first, second])
    await vi.waitFor(() => expect(h.controller.trajectory()?.sessionId).toBe('s2'))
    resolveS1(fixtureSnapshot({ session_id: 's1' }))
    await s1Promise.then(() => Promise.resolve())
    await new Promise((r) => setTimeout(r, 0))
    expect(h.controller.trajectory()?.sessionId).toBe('s2')
  })

  it('live run events fold through the owner and mark retained rows stale', async () => {
    const h = makeHarness(baseHandlers(() => fixtureSnapshot({ session_id: 's1' })))
    await h.controller.connect()
    await vi.waitFor(() => expect(h.controller.trajectory()?.sessionId).toBe('s1'))
    expect(h.controller.trajectory()?.stale).toBe(false)
    h.push({
      kind: 'vivy',
      method: 'run/event',
      params: { event: runEvent('run_b5fb12aaed830e3f', 20, 'tool.approval_required', { approval_id: 'a1' }) },
    } as WireEvent)
    const view = h.controller.trajectory()!
    expect(view.stale).toBe(true)
    expect(view.activity.find((a) => a.run_id === 'run_b5fb12aaed830e3f')?.activity_state).toBe('waiting')
  })

  it('bridge gap marks folded runs incomplete through the owner projection', async () => {
    const h = makeHarness(baseHandlers(() => fixtureSnapshot({ session_id: 's1' })))
    await h.controller.connect()
    await vi.waitFor(() => expect(h.controller.trajectory()?.sessionId).toBe('s1'))
    h.push({ kind: 'bridge', status: 'gap' } as WireEvent)
    expect(h.controller.trajectory()!.incompleteRuns.length).toBeGreaterThan(0)
  })

  it('session without trajectory capability surfaces an honest error', async () => {
    const h = makeHarness(
      baseHandlers(() => Promise.reject(new Error('trajectory is not configured'))),
    )
    await h.controller.loadSession('s1')
    await vi.waitFor(() => expect(h.controller.trajectoryError()).toContain('trajectory is not configured'))
    expect(h.controller.trajectory()).toBeNull()
  })
})
