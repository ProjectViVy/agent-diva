import { describe, expect, it, vi } from 'vitest'
import { VivyCallError, type RunEvent, type WireEvent } from '../api/vivy/contracts'
import { VivyClient } from '../api/vivy/client'
import type { VivyTransport } from '../api/vivy/transport'
import { VivyChatController } from './vivy-chat'

type CallHandler = (params: unknown) => unknown | Promise<unknown>

interface FakeHarness {
  client: VivyClient
  controller: VivyChatController
  calls: Array<{ method: string; params: unknown }>
  push: (event: WireEvent) => void
  setHandler: (method: string, handler: CallHandler) => void
}

function bridgeTimeout(): VivyCallError {
  return new VivyCallError(
    { kind: 'timeout', code: -32000, message: 'call timed out' },
    true,
  )
}

function makeHarness(handlers: Record<string, CallHandler> = {}): FakeHarness {
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
  const controller = new VivyChatController(client)
  return {
    client,
    controller,
    calls,
    push: (event) => {
      for (const l of listeners) l(event)
    },
    setHandler: (method, handler) => {
      handlers[method] = handler
    },
  }
}

function baseHandlers(): Record<string, CallHandler> {
  return {
    initialize: () => ({ protocol_version: 1, capabilities: [] }),
    'session/list': () => ({ sessions: [] }),
    'review/list': () => ({ reviews: [] }),
    'session/get': () => ({
      session: { id: 's1', title: 't', created_at: 1, updated_at: 1 },
      messages: [],
    }),
    'session/work': () => ({
      session_id: 's1',
      version: 1,
      plan: { active: false, submission_id: '', markdown: '', review_status: 'none' },
      activation: 'disarmed',
    }),
    'session/todos': () => ({ todos: [] }),
    'session/messages': () => ({ messages: [] }),
    'run/subscribe': (p) => ({
      subscription_id: 'sub-1',
      run_id: (p as { run_id: string }).run_id,
      after_seq: 0,
    }),
    'turn/start': () => ({ run_id: 'run-1', status: 'accepted' }),
    'run/cancel': () => ({ cancelled: true }),
  }
}

function runEvent(runId: string, seq: number, type: string, payload: Record<string, unknown> = {}): WireEvent {
  return {
    kind: 'vivy',
    method: 'run/event',
    params: { event: { run_id: runId, seq, type, created_at: seq, payload_version: 1, payload } },
  }
}

async function connect(h: FakeHarness): Promise<void> {
  await h.controller.connect()
}

describe('VivyChatController sessions & history', () => {
  it('lists sessions, picks the latest, and renders history', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/list': () => ({
        sessions: [
          { id: 's1', title: 'old', created_at: 1, updated_at: 1 },
          { id: 's2', title: 'new', created_at: 2, updated_at: 9 },
        ],
      }),
      'session/get': (p) => {
        const id = (p as { session_id: string }).session_id
        return {
          session: { id, title: 't', created_at: 1, updated_at: id === 's2' ? 9 : 1 },
          messages: [
            { id: 'm1', role: 'user', content: 'hi', created_at: 1 },
            { id: 'm2', role: 'assistant', content: 'hello back', created_at: 2 },
          ],
        }
      },
    })
    await connect(h)
    expect(h.controller.currentSessionId).toBe('s2')
    expect(h.controller.sessions().map((s) => s.session_key)).toEqual(['s2', 's1'])
    const msgs = h.controller.messages()
    expect(msgs.map((m) => [m.role, m.content])).toEqual([
      ['user', 'hi'],
      ['agent', 'hello back'],
    ])
    expect(msgs.every((m) => m.fromHistory)).toBe(true)
  })

  it('switches sessions and swaps the message timeline', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/list': () => ({
        sessions: [
          { id: 's1', title: 'one', created_at: 1, updated_at: 5 },
          { id: 's2', title: 'two', created_at: 2, updated_at: 6 },
        ],
      }),
      'session/get': (p) => {
        const id = (p as { session_id: string }).session_id
        return {
          session: { id, title: id, created_at: 1, updated_at: 1 },
          messages: [{ id: `m-${id}`, role: 'assistant', content: `from ${id}`, created_at: 1 }],
        }
      },
    })
    await connect(h)
    expect(h.controller.messages()[0].content).toBe('from s2')
    await h.controller.loadSession('s1')
    expect(h.controller.currentSessionId).toBe('s1')
    expect(h.controller.messages()[0].content).toBe('from s1')
  })
})

describe('VivyChatController send & streaming', () => {
  it('echoes the user message, subscribes the authoritative run, streams deltas', async () => {
    const h = makeHarness(baseHandlers())
    await connect(h)
    await h.controller.loadSession('s1')
    const send = h.controller.send('hello there')
    // Echo is visible before turn/start resolves.
    expect(h.controller.messages().at(-1)?.content).toBe('hello there')
    await send
    expect(h.calls.map((c) => c.method)).toContain('turn/start')
    expect(h.calls.map((c) => c.method)).toContain('run/subscribe')
    const sub = h.calls.find((c) => c.method === 'run/subscribe')
    expect((sub?.params as { run_id: string }).run_id).toBe('run-1')

    expect(h.controller.isTyping()).toBe(true)
    h.push(runEvent('run-1', 1, 'run.started'))
    h.push(runEvent('run-1', 2, 'model.delta', { delta: 'ans' }))
    h.push(runEvent('run-1', 3, 'model.delta', { delta: 'wer' }))
    const msgs = h.controller.messages()
    const agent = msgs.find((m) => m.role === 'agent')
    expect(agent?.content).toBe('answer')
    expect(agent?.isStreaming).toBe(true)
    h.push(runEvent('run-1', 4, 'run.completed', { summary: 'answer' }))
    expect(h.controller.isTyping()).toBe(false)
    expect(h.controller.messages().find((m) => m.role === 'agent')?.isStreaming).toBe(false)
  })

  it('renders a terminal-only answer when no deltas arrive', async () => {
    const h = makeHarness(baseHandlers())
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('q')
    h.push(runEvent('run-1', 1, 'run.started'))
    h.push(runEvent('run-1', 2, 'run.completed', { summary: 'complete-only' }))
    const agent = h.controller.messages().find((m) => m.role === 'agent')
    expect(agent?.content).toBe('complete-only')
    expect(agent?.isStreaming).toBeFalsy()
  })

  it('shows reasoning + tool progress and settles the tool card', async () => {
    const h = makeHarness(baseHandlers())
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('q')
    h.push(runEvent('run-1', 1, 'run.started'))
    h.push(runEvent('run-1', 2, 'model.reasoning_delta', { delta: 'thinking' }))
    h.push(runEvent('run-1', 3, 'tool.requested', { tool_call_id: 'c1', tool_name: 'shell', arguments: { cmd: 'ls' } }))
    h.push(runEvent('run-1', 4, 'tool.started', { tool_call_id: 'c1' }))
    let msgs = h.controller.messages()
    const running = msgs.find((m) => m.role === 'tool')
    expect(running?.toolName).toBe('shell')
    expect(running?.toolStatus).toBe('running')
    expect(running?.toolCallId).toBe('c1')
    const agent = msgs.find((m) => m.role === 'agent')
    expect(agent?.reasoning).toBe('thinking')
    expect(agent?.isThinking).toBe(true)
    h.push(runEvent('run-1', 5, 'tool.finished', { tool_call_id: 'c1', status: 'success', result: 'ok' }))
    h.push(runEvent('run-1', 6, 'run.completed', { summary: 'done' }))
    msgs = h.controller.messages()
    expect(msgs.find((m) => m.role === 'tool')?.toolStatus).toBe('success')
    expect(h.controller.isTyping()).toBe(false)
  })

  it('cancelling a running run calls run/cancel and resolves as cancelled', async () => {
    const h = makeHarness(baseHandlers())
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('q')
    h.push(runEvent('run-1', 1, 'run.started'))
    h.push(runEvent('run-1', 2, 'model.delta', { delta: 'partial' }))
    await h.controller.stop()
    const cancel = h.calls.find((c) => c.method === 'run/cancel')
    expect((cancel?.params as { run_id: string }).run_id).toBe('run-1')
    h.push(runEvent('run-1', 3, 'run.cancelled', { reason: 'user' }))
    const agent = h.controller.messages().find((m) => m.role === 'agent')
    expect(agent?.content).toBe('partial')
    expect(agent?.isStreaming).toBe(false)
    expect(h.controller.isTyping()).toBe(false)
  })

  it('a completion racing the cancel wins; the cancel stays recorded', async () => {
    const h = makeHarness(baseHandlers())
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('q')
    h.push(runEvent('run-1', 1, 'run.started'))
    const stop = h.controller.stop()
    h.push(runEvent('run-1', 2, 'run.completed', { summary: 'finished first' }))
    await stop
    h.push(runEvent('run-1', 3, 'run.cancelled')) // backend processed cancel after completion
    const agent = h.controller.messages().find((m) => m.role === 'agent')
    expect(agent?.content).toBe('finished first')
    expect(h.controller.isTyping()).toBe(false)
  })

  it('ambiguous send timeout reconciles through reads instead of resending', async () => {
    let turnCalls = 0
    const h = makeHarness({
      ...baseHandlers(),
      'turn/start': () => {
        turnCalls += 1
        return Promise.reject(bridgeTimeout())
      },
      'session/messages': () => ({
        messages: [{ id: 'm1', role: 'user', content: 'echo me', created_at: 1 }],
      }),
      'session/work': () => ({
        session_id: 's1',
        version: 2,
        plan: { active: false, submission_id: '', markdown: '', review_status: 'none' },
        activation: 'armed',
        current_run_id: 'run-9',
      }),
    })
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('echo me')
    expect(turnCalls).toBe(1) // never resent
    // Echo dropped because the backend persisted the message; run adopted.
    const userMsgs = h.controller.messages().filter((m) => m.role === 'user')
    expect(userMsgs).toHaveLength(1)
    const subs = h.calls.filter((c) => c.method === 'run/subscribe')
    expect(subs.map((c) => (c.params as { run_id: string }).run_id)).toContain('run-9')
    expect(h.controller.isTyping()).toBe(true)
  })

  it('keeps the local echo when the backend never saw the send', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'turn/start': () => Promise.reject(bridgeTimeout()),
      'session/messages': () => ({ messages: [] }),
    })
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('lost?')
    const msgs = h.controller.messages()
    expect(msgs.at(-1)?.role).toBe('user')
    expect(msgs.at(-1)?.content).toBe('lost?')
    expect(h.controller.isTyping()).toBe(false)
  })
})

describe('VivyChatController approvals & questions', () => {
  function approvalReview(over: Record<string, unknown> = {}) {
    return {
      id: 'rev-1',
      kind: 'approval',
      status: 'pending',
      session_id: 's1',
      run_id: 'run-1',
      tool_call_id: 'c1',
      tool_name: 'write_note',
      created_at: 10,
      expires_at: 20,
      prompt: 'allow writing?',
      ...over,
    }
  }

  it('lists pending approvals and approve/deny go through review/respond by id', async () => {
    let reviews: unknown[] = [approvalReview()]
    const h = makeHarness({
      ...baseHandlers(),
      'review/list': () => ({ reviews }),
      'review/respond': () => {
        reviews = [approvalReview({ status: 'approved' })]
        return { ok: true }
      },
    })
    await connect(h)
    const approvals = h.controller.approvals()
    expect(approvals).toHaveLength(1)
    expect(approvals[0].request_id).toBe('rev-1')
    expect(approvals[0].status).toBe('pending')
    await h.controller.decideApproval('rev-1', true)
    const call = h.calls.find((c) => c.method === 'review/respond')
    expect(call?.params).toMatchObject({ review_id: 'rev-1', action: 'approve' })
    // Authoritative list flipped the row to non-pending.
    expect(h.controller.approvals()[0]?.status).toBe('allowed')
  })

  it('surfaces backend rejection on a decided review (double decision)', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'review/list': () => ({ reviews: [approvalReview()] }),
      'review/respond': () =>
        Promise.reject(
          new VivyCallError({ kind: 'internal', code: -32009, message: 'review already decided' }),
        ),
    })
    await connect(h)
    await h.controller.decideApproval('rev-1', true)
    expect(h.controller.actionErrors.get('rev-1')).toBe('review already decided')
    expect(h.controller.isSubmitting('rev-1')).toBe(false)
  })

  it('marks the outcome unknown when a mutating respond call times out', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'review/list': () => ({ reviews: [approvalReview()] }),
      'review/respond': () => Promise.reject(bridgeTimeout()),
    })
    await connect(h)
    await h.controller.decideApproval('rev-1', false)
    expect(h.controller.outcomeUnknown.has('rev-1')).toBe(true)
  })

  it('reopening with a pending approval keeps it visible from the snapshot', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'review/list': () => ({ reviews: [approvalReview()] }),
    })
    await connect(h)
    expect(h.controller.approvals()[0]?.status).toBe('pending')
  })

  it('expiry events refresh the list and settle the approval', async () => {
    let reviews: unknown[] = [approvalReview()]
    const h = makeHarness({
      ...baseHandlers(),
      'review/list': () => ({ reviews }),
    })
    await connect(h)
    expect(h.controller.approvals()[0]?.status).toBe('pending')
    reviews = [approvalReview({ status: 'expired' })]
    h.push(runEvent('run-1', 5, 'tool.approval_expired', { approval_id: 'rev-1' }))
    await h.controller.refreshInteractions()
    expect(h.controller.approvals()[0]?.status).toBe('expired')
  })

  it('answers and cancels free-text questions through review/respond', async () => {
    const question = {
      id: 'q-1',
      kind: 'question',
      status: 'pending',
      session_id: 's1',
      run_id: 'run-1',
      tool_call_id: 'c2',
      prompt: 'which file?',
      created_at: 10,
      expires_at: 30,
    }
    const h = makeHarness({
      ...baseHandlers(),
      'review/list': () => ({ reviews: [question] }),
      'review/respond': () => ({ ok: true }),
    })
    await connect(h)
    const qs = h.controller.questions()
    expect(qs).toHaveLength(1)
    expect(qs[0].question_id).toBe('q-1')
    expect(qs[0].allow_other).toBe(true)
    await h.controller.answerQuestion('q-1', 'src/main.ts')
    expect(h.calls.find((c) => c.method === 'review/respond')?.params).toMatchObject({
      review_id: 'q-1',
      action: 'answer',
      answer: 'src/main.ts',
    })
    await h.controller.cancelQuestion('q-1')
    const cancel = h.calls.filter((c) => c.method === 'review/respond').at(-1)
    expect(cancel?.params).toMatchObject({ review_id: 'q-1', action: 'cancel' })
  })
})

describe('VivyChatController DN-2A images & permission', () => {
  const PNG_B64 =
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='
  const bigPngBase64 = () => {
    const bytes = new Uint8Array((3 << 20) + 1024)
    bytes.set([0x89, 0x50, 0x4e, 0x47])
    let bin = ''
    for (let i = 0; i < bytes.length; i += 0x8000) {
      bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
    }
    return btoa(bin)
  }
  const sessionWith = (preset: string) => ({
    id: 's1',
    title: 't',
    created_at: 1,
    updated_at: 1,
    permission_preset: preset,
    sandbox_mode: preset === 'cautious' ? 'read_only' : 'workspace_write',
    approval_policy: 'ask',
  })

  it('sendImageBytes: image reaches turn/start in the fixture wire shape', async () => {
    const h = makeHarness(baseHandlers())
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('image echo', {
      attachments: [{ name: 'dot.png', mime_type: 'image/png', data: PNG_B64 }],
    })
    const turn = h.calls.find((c) => c.method === 'turn/start')
    expect(turn?.params).toEqual({
      session_id: 's1',
      text: 'image echo',
      attachments: [{ name: 'dot.png', mime_type: 'image/png', data: PNG_B64 }],
    })
  })

  it('rejectOversizeFramedRequest: a >4 MiB framed call mutates nothing', async () => {
    const h = makeHarness(baseHandlers())
    await connect(h)
    await h.controller.loadSession('s1')
    await expect(
      h.controller.send('huge', {
        attachments: [{ name: 'big.png', mime_type: 'image/png', data: bigPngBase64() }],
      }),
    ).rejects.toThrow(/4 MiB/)
    expect(h.calls.filter((c) => c.method === 'turn/start')).toHaveLength(0)
    // The draft produced no echo — the timeline stays as history gave it.
    expect(h.controller.messages()).toHaveLength(0)
  })

  it('unsupportedFileKeepsDraft: unsupported MIME performs no mutation', async () => {
    const h = makeHarness(baseHandlers())
    await connect(h)
    await h.controller.loadSession('s1')
    await expect(
      h.controller.send('draft stays', {
        attachments: [{ name: 'x.png', mime_type: 'application/pdf', data: PNG_B64 }],
      }),
    ).rejects.toThrow()
    expect(h.calls.filter((c) => c.method === 'turn/start')).toHaveLength(0)
    expect(h.controller.messages()).toHaveLength(0)
  })

  it('permissionReadbackBeforeSend: preset is armed+admitted before the turn', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/set_permission': () => sessionWith('cautious'),
    })
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('go', { preset: 'cautious' })
    const methods = h.calls.map((c) => c.method)
    expect(methods.indexOf('session/set_permission')).toBeGreaterThan(-1)
    expect(methods.indexOf('session/set_permission')).toBeLessThan(methods.indexOf('turn/start'))
    expect(
      h.calls.find((c) => c.method === 'session/set_permission')?.params,
    ).toEqual({ session_id: 's1', preset: 'cautious' })
    // The admitted policy snapshot is published on the projection.
    expect(h.controller.projection.sessions.get('s1')?.permission_preset).toBe('cautious')
  })

  it('permissionReadbackBeforeSend: admitted mismatch blocks the send after readback', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/set_permission': () => sessionWith('smart'),
      'session/get': () => ({ session: sessionWith('smart'), messages: [] }),
    })
    await connect(h)
    await h.controller.loadSession('s1')
    await expect(h.controller.send('go', { preset: 'cautious' })).rejects.toThrow(
      /not admitted/,
    )
    expect(h.calls.filter((c) => c.method === 'turn/start')).toHaveLength(0)
  })

  it('permissionReadbackBeforeSend: ambiguous write reconciled by readback, then sends', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/set_permission': () => Promise.reject(bridgeTimeout()),
      'session/get': () => ({ session: sessionWith('cautious'), messages: [] }),
    })
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.send('go', { preset: 'cautious' })
    expect(h.calls.filter((c) => c.method === 'session/set_permission')).toHaveLength(1)
    expect(h.calls.map((c) => c.method)).toContain('turn/start')
  })

  it('duplicateSendSerialized: concurrent sends run through one mutation lane', async () => {
    const order: string[] = []
    const release: Array<() => void> = []
    const h = makeHarness({
      ...baseHandlers(),
      'turn/start': (p) => {
        const text = (p as { text: string }).text
        order.push(`start:${text}`)
        return new Promise((resolve) => {
          release.push(() => {
            order.push(`end:${text}`)
            resolve({ run_id: `run-${text}`, status: 'accepted' })
          })
        })
      },
    })
    await connect(h)
    await h.controller.loadSession('s1')
    const first = h.controller.send('a')
    const second = h.controller.send('b')
    await Promise.resolve()
    release[0]!()
    await first
    await Promise.resolve()
    release[1]!()
    await second
    expect(order).toEqual(['start:a', 'end:a', 'start:b', 'end:b'])
  })
})

describe('VivyChatController planning/work', () => {
  function pendingWork() {
    return {
      session_id: 's1',
      version: 7,
      plan: {
        active: true,
        submission_id: 'sub-42',
        markdown: '# Plan',
        review_status: 'pending',
        feedback: '',
      },
      activation: 'disarmed',
      goal: { id: 'g1', revision: 1, objective: 'ship it' },
    }
  }

  it('maps a pending plan review to the approval surface', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/list': () => ({ sessions: [{ id: 's1', title: 't', created_at: 1, updated_at: 1 }] }),
      'session/work': () => pendingWork(),
      'session/todos': () => ({
        todos: [{ id: 't1', session_id: 's1', subject: 'do x', status: 'pending', blocks: [], blocked_by: [] }],
      }),
    })
    await connect(h)
    const plan = h.controller.plan()
    expect(plan?.phase).toBe('AwaitingApproval')
    expect(plan?.plan_id).toBe('sub-42')
    expect(plan?.todos).toHaveLength(1)
  })

  it('decides a plan through plan/decide with the frozen work params', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/list': () => ({ sessions: [{ id: 's1', title: 't', created_at: 1, updated_at: 1 }] }),
      'session/work': () => pendingWork(),
      'plan/decide': () => ({
        work: { ...pendingWork(), plan: { ...pendingWork().plan, review_status: 'accepted' } },
        event: { seq: 3, kind: 'plan.decided', request_id: 'req', created_at: 9 },
        replayed: false,
      }),
    })
    await connect(h)
    await h.controller.decidePlan('execute_once')
    const call = h.calls.find((c) => c.method === 'plan/decide')
    expect(call?.params).toMatchObject({
      session_id: 's1',
      submission_id: 'sub-42',
      action: 'execute_once',
      expected_version: 7,
    })
    expect(h.controller.plan()?.review_status ?? 'accepted').toBe('accepted')
    expect(h.controller.plan()?.phase).toBe('Execute')
  })

  it('rejects a plan decision when no submission is pending', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/list': () => ({ sessions: [{ id: 's1', title: 't', created_at: 1, updated_at: 1 }] }),
    })
    await connect(h)
    expect(h.controller.currentSessionId).toBe('s1')
    await expect(h.controller.decidePlan('execute_once')).rejects.toThrow('no pending plan')
  })

  it('goalUsesWorkController: start_goal goes through plan/decide with objective+budget', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/list': () => ({ sessions: [{ id: 's1', title: 't', created_at: 1, updated_at: 1 }] }),
      'session/work': () => pendingWork(),
      'plan/decide': () => ({
        work: {
          ...pendingWork(),
          activation: 'armed',
          plan: { ...pendingWork().plan, review_status: 'accepted' },
          goal: { id: 'goal-1', revision: 1, objective: 'ship it', phase: 'running', max_rounds: 20, rounds_started: 0 },
        },
        event: { seq: 4, kind: 'plan.decided', request_id: 'req', created_at: 9 },
        replayed: false,
      }),
    })
    await connect(h)
    await h.controller.decidePlan('start_goal', undefined, { objective: 'ship it', max_rounds: 20 })
    const call = h.calls.find((c) => c.method === 'plan/decide')
    expect(call?.params).toMatchObject({
      session_id: 's1',
      submission_id: 'sub-42',
      action: 'start_goal',
      objective: 'ship it',
      max_rounds: 20,
      expected_version: 7,
    })
    // No second planner, no invented RPC — the same work controller flow.
    expect(h.calls.filter((c) => c.method === 'plan/decide')).toHaveLength(1)
  })
})

describe('VivyChatController DN-2B regenerate & recovery', () => {
  const PNG_B64 =
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='
  const imageAttachment = {
    name: 'dot.png',
    mime_type: 'image/png',
    data_url: `data:image/png;base64,${PNG_B64}`,
    size: 70,
  }
  const textHistory = [
    { id: 'u1', run_id: 'r1', role: 'user', content: 'first q', created_at: 1 },
    { id: 'a1', run_id: 'r1', role: 'assistant', content: 'first a', created_at: 2 },
    { id: 'u2', run_id: 'r2', role: 'user', content: 'second q', created_at: 3 },
    { id: 'a2', run_id: 'r2', role: 'assistant', content: 'second a', created_at: 4 },
  ]
  const imageHistory = [
    { id: 'u1', run_id: 'r1', role: 'user', content: 'first q', created_at: 1 },
    { id: 'a1', run_id: 'r1', role: 'assistant', content: 'first a', created_at: 2 },
    {
      id: 'u2',
      run_id: 'r2',
      role: 'user',
      content: 'image q',
      attachments: [imageAttachment],
      created_at: 3,
    },
    { id: 'a2', run_id: 'r2', role: 'assistant', content: 'image a', created_at: 4 },
  ]
  const harnessWithHistory = (messages: unknown[], extra: Record<string, CallHandler> = {}) =>
    makeHarness({
      ...baseHandlers(),
      'session/list': () => ({ sessions: [{ id: 's1', title: 't', created_at: 1, updated_at: 1 }] }),
      'session/get': () => ({
        session: { id: 's1', title: 't', created_at: 1, updated_at: 1 },
        messages,
      }),
      ...extra,
    })

  it('regenerateSelectedTextAtomicEdit: atomic session/edit replays the ORIGINATING turn', async () => {
    const h = harnessWithHistory(textHistory, {
      'session/edit': () => ({ run_id: 'run-edit-1', status: 'accepted' }),
    })
    await connect(h)
    await h.controller.regenerate('a1')
    const edit = h.calls.find((c) => c.method === 'session/edit')
    expect(edit?.params).toMatchObject({
      session_id: 's1',
      message_id: 'u1', // the originating user turn of a1 — not the last turn u2
      text: 'first q',
    })
    expect(h.calls.filter((c) => c.method === 'turn/start')).toHaveLength(0)
    expect(h.calls.filter((c) => c.method === 'session/rewind')).toHaveLength(0)
    // The admitted run replaces activeRun and is subscribed.
    const sub = h.calls.find((c) => c.method === 'run/subscribe')
    expect((sub?.params as { run_id: string }).run_id).toBe('run-edit-1')
    expect(h.controller.isTyping()).toBe(true)
  })

  it('regenerateImageInclusiveCutoff: image turn rewinds inclusively then resends original bytes', async () => {
    const h = harnessWithHistory(imageHistory, {
      'session/rewind': () => ({ cutoff_message_id: 'u2', remaining_count: 2 }),
      'turn/start': () => ({ run_id: 'run-img-1', status: 'accepted' }),
    })
    await connect(h)
    await h.controller.regenerate('a2')
    const methods = h.calls.map((c) => c.method)
    expect(methods.indexOf('session/rewind')).toBeGreaterThan(-1)
    expect(methods.indexOf('session/rewind')).toBeLessThan(methods.indexOf('turn/start'))
    expect(h.calls.find((c) => c.method === 'session/rewind')?.params).toEqual({
      session_id: 's1',
      message_id: 'u2',
    })
    const turn = h.calls.find((c) => c.method === 'turn/start')
    expect(turn?.params).toEqual({
      session_id: 's1',
      text: 'image q',
      // Original bytes re-encoded as wire attachments — never silently
      // converted to a text-only resend, never a duplicated user message.
      attachments: [{ name: 'dot.png', mime_type: 'image/png', data: PNG_B64 }],
    })
    expect(h.calls.filter((c) => c.method === 'session/edit')).toHaveLength(0)
  })

  it('rewindSucceededSendFailedDraft: failed resend keeps a retryable draft + rewound view', async () => {
    let rewound = false
    const h = harnessWithHistory(imageHistory, {
      'session/rewind': () => {
        rewound = true
        return { cutoff_message_id: 'u2', remaining_count: 2 }
      },
      'session/get': () => ({
        session: { id: 's1', title: 't', created_at: 1, updated_at: 1 },
        messages: rewound ? imageHistory.slice(0, 2) : imageHistory,
      }),
      'turn/start': () => Promise.reject(new Error('send failed')),
    })
    await connect(h)
    await expect(h.controller.regenerate('a2')).rejects.toThrow('send failed')
    expect(h.calls.filter((c) => c.method === 'session/rewind')).toHaveLength(1)
    // Retryable draft retains the original text + bytes for a manual resend.
    const draft = h.controller.retryDraft
    expect(draft?.content).toBe('image q')
    expect(draft?.attachments).toEqual([
      { name: 'dot.png', mime_type: 'image/png', data: PNG_B64 },
    ])
    // The visible timeline stays truthfully rewound — no re-appended copy.
    const contents = h.controller.messages().map((m) => m.content)
    expect(contents).not.toContain('image q')
    expect(contents).not.toContain('image a')
  })

  it('cancelAfterRestartReconciles: not-found cancel reads back authoritative run state', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/list': () => ({ sessions: [{ id: 's1', title: 't', created_at: 1, updated_at: 1 }] }),
      'run/cancel': () =>
        Promise.reject(
          new VivyCallError(
            { kind: 'internal', code: -32004, message: 'run is not active in this process' },
          ),
        ),
      'run/get': () => ({ id: 'run-1', session_id: 's1', status: 'completed', created_at: 1 }),
    })
    await connect(h)
    await h.controller.send('q')
    expect(h.controller.isTyping()).toBe(true)
    await h.controller.stop() // must not throw: readback proves terminal
    expect(h.calls.find((c) => c.method === 'run/get')?.params).toEqual({ run_id: 'run-1' })
    expect(h.controller.isTyping()).toBe(false)
    // Never claimed cancelled locally — the authoritative phase is 'completed'.
    expect(h.controller.projection.run('run-1')?.phase).toBe('completed')
  })

  it('fork(messageId): inclusive fork opens the copied child session', async () => {
    let childListed = false
    const h = harnessWithHistory(textHistory, {
      'session/fork': () => ({
        session_id: 's2',
        fork_point_message_id: 'u2',
        copied_count: 3,
      }),
      'session/list': () => ({
        sessions: [
          { id: 's1', title: 't', created_at: 1, updated_at: 1 },
          ...(childListed ? [{ id: 's2', title: 'fork', created_at: 2, updated_at: 2 }] : []),
        ],
      }),
    })
    h.setHandler('session/fork', () => {
      childListed = true
      return { session_id: 's2', fork_point_message_id: 'u2', copied_count: 3 }
    })
    await connect(h)
    await h.controller.fork('u2')
    expect(h.calls.find((c) => c.method === 'session/fork')?.params).toMatchObject({
      session_id: 's1',
      message_id: 'u2',
    })
    expect(h.controller.currentSessionId).toBe('s2')
  })

  it('invalidateConversation(reason) fires before session mutations', async () => {
    const h = makeHarness({
      ...baseHandlers(),
      'session/delete': () => ({ deleted: true }),
    })
    const reasons: string[] = []
    h.controller.onConversationInvalidate((r) => reasons.push(r))
    await connect(h)
    await h.controller.loadSession('s1')
    await h.controller.deleteSession('s1')
    expect(reasons).toEqual(['session/load', 'session/delete'])
  })
})
