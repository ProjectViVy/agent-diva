import { describe, expect, it } from 'vitest'
import type { RunEvent, WireEvent } from '../api/vivy/contracts'
import { VivyClient } from '../api/vivy/client'
import type { VivyTransport } from '../api/vivy/transport'
import { VivySessionProjection } from './vivy-session'

function ev(runId: string, seq: number, type: string, payload: Record<string, unknown> = {}): RunEvent {
  return { run_id: runId, seq, type, created_at: seq, payload_version: 1, payload }
}

function note(event: RunEvent): WireEvent {
  return { kind: 'vivy', method: 'run/event', params: { event } }
}

describe('VivySessionProjection reducer', () => {
  it('applies events in order and dedups late/repeated (run_id, seq)', () => {
    const p = new VivySessionProjection()
    p.reduce(note(ev('r1', 1, 'run.started')))
    p.reduce(note(ev('r1', 2, 'model.delta', { delta: 'a' })))
    p.reduce(note(ev('r1', 2, 'model.delta', { delta: 'a' }))) // repeat
    p.reduce(note(ev('r1', 1, 'run.started'))) // late
    const run = p.run('r1')
    expect(run?.cursor).toBe(2)
    expect(run?.events).toHaveLength(2)
    expect(run?.streamText).toBe('a')
    expect(run?.needsResync).toBe(false)
  })

  it('buffers out-of-order events and never advances the cursor across a gap', () => {
    const p = new VivySessionProjection()
    p.reduce(note(ev('r1', 1, 'run.started')))
    p.reduce(note(ev('r1', 3, 'model.delta', { delta: 'late' })))
    let run = p.run('r1')
    expect(run?.cursor).toBe(1)
    expect(run?.bufferedCount).toBe(1)
    expect(run?.needsResync).toBe(true)
    expect(run?.streamText).toBe('')
    // seq 2 arrives later — drain continues in order
    p.reduce(note(ev('r1', 2, 'model.delta', { delta: 'x' })))
    run = p.run('r1')
    expect(run?.cursor).toBe(3)
    expect(run?.bufferedCount).toBe(0)
    expect(run?.streamText).toBe('xlate')
    // still flagged: the gap was real until an authoritative replay
    expect(run?.needsResync).toBe(true)
  })

  it('marks every run needsResync on a bridge gap and tracks lost', () => {
    const p = new VivySessionProjection()
    p.reduce(note(ev('r1', 1, 'run.started')))
    p.reduce(note(ev('r2', 1, 'run.started')))
    p.reduce({ kind: 'bridge', status: 'gap' })
    expect(p.connection).toBe('gap')
    expect(p.run('r1')?.needsResync).toBe(true)
    expect(p.run('r2')?.needsResync).toBe(true)
    p.reduce({ kind: 'bridge', status: 'lost' })
    expect(p.connection).toBe('lost')
  })

  it('answers terminal-only runs without deltas', () => {
    const p = new VivySessionProjection()
    p.reduce(note(ev('r1', 1, 'run.started')))
    p.reduce(note(ev('r1', 2, 'run.completed', { summary: 'final answer' })))
    const run = p.run('r1')
    expect(run?.phase).toBe('completed')
    expect(p.answer('r1')).toBe('final answer')
  })

  it('is idempotent under two-window replay of the same events', () => {
    const p = new VivySessionProjection()
    const events = [
      note(ev('r1', 1, 'run.started')),
      note(ev('r1', 2, 'model.delta', { delta: 'hi' })),
      note(ev('r1', 3, 'run.completed', { summary: 'hi' })),
    ]
    for (const e of events) p.reduce(e)
    for (const e of events) p.reduce(e) // second window replays the same stream
    const run = p.run('r1')
    expect(run?.events).toHaveLength(3)
    expect(run?.cursor).toBe(3)
    expect(p.answer('r1')).toBe('hi')
  })

  it('tracks pending approvals from events and clears on decision', () => {
    const p = new VivySessionProjection()
    p.reduce(note(ev('r1', 1, 'run.started')))
    p.reduce(
      note(ev('r1', 2, 'tool.approval_required', {
        approval_id: 'a1', tool_call_id: 'c1', tool_name: 'write_note',
      })),
    )
    expect(p.pendingInteractions()).toEqual([
      expect.objectContaining({ kind: 'approval', id: 'a1', runId: 'r1', toolCallId: 'c1' }),
    ])
    p.reduce(note(ev('r1', 3, 'tool.approval_decided', { approval_id: 'a1', decision: 'approved' })))
    expect(p.pendingInteractions()).toEqual([])
  })

  it('reconstructs pending interactions from snapshots (window reopen)', () => {
    const p = new VivySessionProjection()
    // Reopen path: no approval_required event seen, snapshot supplies it.
    p.applySnapshot({
      type: 'approval/list',
      result: {
        approvals: [{ id: 'a9', run_id: 'r9', tool_call_id: 'c9', decision: 'pending', expires_at: 5 }],
      },
    })
    expect(p.pendingInteractions()).toEqual([
      expect.objectContaining({ kind: 'approval', id: 'a9', runId: 'r9' }),
    ])
  })

  it('snapshot approval list is authoritative: decisions made elsewhere disappear', () => {
    const p = new VivySessionProjection()
    p.reduce(note(ev('r1', 1, 'run.started')))
    p.reduce(note(ev('r1', 2, 'tool.approval_required', { approval_id: 'a1' })))
    p.reduce(note(ev('r1', 3, 'tool.approval_required', { approval_id: 'a2' })))
    // second window approved a1 — resync snapshot shows only a2 pending
    p.applySnapshot({
      type: 'approval/list',
      result: { approvals: [{ id: 'a2', run_id: 'r1', tool_call_id: 'c2', decision: 'pending' }] },
    })
    expect(p.pendingInteractions().map((x) => x.id)).toEqual(['a2'])
  })

  it('run/log snapshot replays authoritatively and keeps holes flagged', () => {
    const p = new VivySessionProjection()
    p.reduce(note(ev('r1', 1, 'run.started')))
    p.reduce(note(ev('r1', 4, 'model.delta', { delta: 'd4' })))
    p.applySnapshot({
      type: 'run/log',
      result: {
        events: [
          ev('r1', 1, 'run.started'),
          ev('r1', 2, 'model.delta', { delta: 'd2' }),
          ev('r1', 3, 'model.delta', { delta: 'd3' }),
          ev('r1', 4, 'model.delta', { delta: 'd4' }),
        ],
      },
    })
    const run = p.run('r1')
    expect(run?.cursor).toBe(4)
    expect(run?.streamText).toBe('d2d3d4')
    expect(run?.needsResync).toBe(false)
  })
})

describe('VivySessionProjection recovery', () => {
  function clientWith(log: RunEvent[], approvals: unknown[] = []): VivyClient {
    const t: VivyTransport = {
      call: (req) => {
        if (req.method === 'run/log') return Promise.resolve({ events: log })
        if (req.method === 'run/get')
          return Promise.resolve({ id: 'r1', session_id: 's1', status: 'active', created_at: 1 })
        if (req.method === 'approval/list') return Promise.resolve({ approvals })
        return Promise.reject(new Error(`unexpected call ${req.method}`))
      },
      onEvent: () => Promise.resolve(() => {}),
      close: () => {},
    }
    return new VivyClient(t)
  }

  it('attaches the listener before snapshots and merges buffered events on resync', async () => {
    const p = new VivySessionProjection()
    const client = clientWith([
      ev('r1', 1, 'run.started'),
      ev('r1', 2, 'model.delta', { delta: 'a' }),
      ev('r1', 3, 'model.delta', { delta: 'b' }),
    ])
    await p.attach(client) // listener first
    // live event seq 3 arrives before the snapshot is read
    p.reduce(note(ev('r1', 3, 'model.delta', { delta: 'b' })))
    expect(p.run('r1')?.cursor).toBe(0)
    await p.resync()
    const run = p.run('r1')
    expect(run?.cursor).toBe(3)
    expect(run?.streamText).toBe('ab')
    expect(run?.needsResync).toBe(false)
    expect(p.connection).toBe('connected')
  })

  it('detach stops reducing and marks disconnected', async () => {
    const p = new VivySessionProjection()
    await p.attach(clientWith([]))
    expect(p.connection).toBe('connected')
    p.detach()
    expect(p.connection).toBe('disconnected')
  })
})
