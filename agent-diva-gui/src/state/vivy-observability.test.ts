import { describe, expect, it } from 'vitest'
import type { VivyTokenUsageSnapshot, WireEvent } from '../api/vivy/contracts'
import { VivyClient } from '../api/vivy/client'
import type { VivyTransport } from '../api/vivy/transport'
import { costDisplay, usageView, VivyObservabilityController } from './vivy-observability'

type CallHandler = (params: unknown) => unknown | Promise<unknown>

interface Harness {
  client: VivyClient
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
  return {
    client: new VivyClient(transport),
    calls,
    push: (event) => {
      for (const l of listeners) l(event)
    },
    setHandler: (method, handler) => {
      handlers[method] = handler
    },
  }
}

function coverage(over: Record<string, unknown> = {}) {
  return {
    state: 'partial',
    observed_calls: 3,
    completed_with_usage: 0,
    reported_calls: 0,
    missing_usage_calls: 3,
    partial_usage_calls: 0,
    active_calls: 0,
    legacy_usage_records: 0,
    unknown_buckets: [],
    hidden_retries_observable: false,
    ...over,
  }
}

/** Verbatim v2 shape from closure-chat-obs.json requests_responses[47]. */
function snapshot(over: Partial<VivyTokenUsageSnapshot> = {}): VivyTokenUsageSnapshot {
  return {
    period: '1d',
    scope: 'chat_runs',
    projection_version: 2,
    coverage: coverage(),
    total: {
      total_input: 0,
      total_output: 0,
      total_tokens: 0,
      total_reasoning: 0,
      total_cached: 0,
      request_count: 0,
      total_cost_usd: 0,
      cost_known: false,
    },
    models: [],
    providers: [],
    timeline: [],
    sessions: [],
    ...over,
  } as VivyTokenUsageSnapshot
}

function deferred<T>() {
  let resolve!: (v: T) => void
  let reject!: (e: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

describe('usageView coverage mapping (C2-1 fields)', () => {
  it('maps nil usage to empty — not a zero total', () => {
    const view = usageView(snapshot({ coverage: coverage({ state: 'empty', observed_calls: 0 }) }))
    expect(view.state).toBe('empty')
  })

  it('maps complete coverage when all observed calls reported', () => {
    const view = usageView(
      snapshot({
        coverage: coverage({
          state: 'complete',
          observed_calls: 2,
          completed_with_usage: 2,
          reported_calls: 2,
          missing_usage_calls: 0,
        }),
      }),
    )
    expect(view.state).toBe('complete')
  })

  it('keeps partial distinct: missing/active/partial partitions are not completed', () => {
    const view = usageView(
      snapshot({
        coverage: coverage({
          state: 'partial',
          observed_calls: 6,
          completed_with_usage: 2,
          reported_calls: 3,
          partial_usage_calls: 1,
          missing_usage_calls: 2,
          active_calls: 1,
        }),
      }),
    )
    expect(view.state).toBe('partial')
    const c = view.coverage
    // observed partition only — reported_calls is not an extra partition.
    expect(c.completed_with_usage + c.partial_usage_calls + c.missing_usage_calls + c.active_calls).toBe(
      c.observed_calls,
    )
    expect(c.reported_calls).toBe(3)
  })

  it('maps legacy-only evidence to legacy', () => {
    const view = usageView(
      snapshot({ coverage: coverage({ state: 'legacy', observed_calls: 0, legacy_usage_records: 4 }) }),
    )
    expect(view.state).toBe('legacy')
    expect(view.coverage.legacy_usage_records).toBe(4)
  })

  it('surfaces unknown buckets and hides nothing about hidden retries', () => {
    const view = usageView(
      snapshot({ coverage: coverage({ state: 'partial', unknown_buckets: ['reasoning', 'cached'] }) }),
    )
    expect(view.unknownBuckets).toEqual(['reasoning', 'cached'])
    expect(view.coverage.hidden_retries_observable).toBe(false)
  })

  it('reported_calls may count active calls — never added to the partition', () => {
    const view = usageView(
      snapshot({
        coverage: coverage({
          state: 'partial',
          observed_calls: 2,
          completed_with_usage: 0,
          reported_calls: 1,
          active_calls: 1,
          partial_usage_calls: 1,
          missing_usage_calls: 0,
        }),
      }),
    )
    expect(view.coverage.reported_calls).toBe(1)
  })
})

describe('costDisplay', () => {
  it('cost_known:false is unknown — never renders as free or $0.00', () => {
    expect(costDisplay(0, false)).toEqual({ kind: 'unknown' })
    expect(costDisplay(0.0042, false)).toEqual({ kind: 'unknown' })
  })

  it('cost_known:true renders the reported reference cost', () => {
    expect(costDisplay(1.234, true)).toEqual({ kind: 'known', usd: 1.234 })
    expect(costDisplay(0, true)).toEqual({ kind: 'known', usd: 0 })
  })
})

describe('VivyObservabilityController host connection', () => {
  it('preview shell never issues RPCs and exposes no host metadata', async () => {
    const h = makeHarness()
    const obs = new VivyObservabilityController(() => false)
    await obs.attach(h.client)
    expect(obs.connection().state).toBe('preview')
    expect(obs.connection().protocolVersion).toBeUndefined()
    const res = await obs.refresh({ period: '1d' })
    expect(res).toBeNull()
    expect(h.calls).toEqual([])
  })

  it('connected host exposes initialize metadata, not a health RPC', async () => {
    const h = makeHarness({
      initialize: () => ({ protocol_version: '1', capabilities: ['stats/tokens'] }),
    })
    const obs = new VivyObservabilityController(() => true)
    await obs.attach(h.client)
    expect(obs.connection().state).toBe('connected')
    expect(obs.connection().protocolVersion).toBe('1')
    expect(obs.connection().capabilities).toContain('stats/tokens')
    // exactly the listener + initialize — no extra polling call
    expect(h.calls.map((c) => c.method)).toEqual(['initialize'])
  })

  it('initialize failure is unavailable, not connected', async () => {
    const h = makeHarness({ initialize: () => Promise.reject(new Error('ABI mismatch')) })
    const obs = new VivyObservabilityController(() => true)
    await obs.attach(h.client)
    expect(obs.connection().state).toBe('unavailable')
    expect(obs.connection().protocolVersion).toBeUndefined()
  })

  it('bridge gap/lost degrade the reported connection and stale the totals', async () => {
    const h = makeHarness({
      initialize: () => ({ protocol_version: '1', capabilities: [] }),
      'stats/tokens': () => snapshot(),
    })
    const obs = new VivyObservabilityController(() => true)
    await obs.attach(h.client)
    await obs.refresh({ period: '1d' })
    expect(obs.stale()).toBe(false)
    h.push({ kind: 'bridge', status: 'gap' } as WireEvent)
    expect(obs.connection().state).toBe('gap')
    expect(obs.stale()).toBe(true)
    h.push({ kind: 'bridge', status: 'lost' } as WireEvent)
    expect(obs.connection().state).toBe('lost')
  })

  it('detach clears host metadata — reconnect shows connecting, not stale live state', async () => {
    const h = makeHarness({ initialize: () => ({ protocol_version: '1', capabilities: [] }) })
    const obs = new VivyObservabilityController(() => true)
    await obs.attach(h.client)
    obs.detach()
    expect(obs.connection().state).toBe('disconnected')
    expect(obs.connection().protocolVersion).toBeUndefined()
  })
})

describe('VivyObservabilityController refresh fencing', () => {
  const attachConnected = async () => {
    const h = makeHarness({
      initialize: () => ({ protocol_version: '1', capabilities: [] }),
      'stats/tokens': () => snapshot(),
    })
    const obs = new VivyObservabilityController(() => true)
    await obs.attach(h.client)
    return { h, obs }
  }

  it('coalesces one in-flight snapshot request — no duplicate RPC', async () => {
    const h = makeHarness({
      initialize: () => ({ protocol_version: '1', capabilities: [] }),
      'stats/tokens': () => deferred<VivyTokenUsageSnapshot>().promise,
    })
    const obs = new VivyObservabilityController(() => true)
    await obs.attach(h.client)
    void obs.refresh({ period: '1d' })
    void obs.refresh({ period: '1d' })
    expect(h.calls.filter((c) => c.method === 'stats/tokens')).toHaveLength(1)
  })

  it('a delayed response after a param switch cannot replace current totals', async () => {
    const slow = deferred<VivyTokenUsageSnapshot>()
    const h = makeHarness({
      initialize: () => ({ protocol_version: '1', capabilities: [] }),
      'stats/tokens': (params) => {
        const p = params as { period?: string }
        if (p.period === '1d') return slow.promise
        return Promise.resolve(snapshot({ period: '3d' }))
      },
    })
    const obs = new VivyObservabilityController(() => true)
    await obs.attach(h.client)
    const staleReq = obs.refresh({ period: '1d' })
    const fresh = await obs.refresh({ period: '3d' })
    expect(fresh?.period).toBe('3d')
    slow.resolve(snapshot({ period: '1d' }))
    expect(await staleReq).toBeNull()
    expect(obs.snapshot()?.period).toBe('3d')
  })

  it('reconnect invalidates in-flight responses', async () => {
    const pending = deferred<VivyTokenUsageSnapshot>()
    const h = makeHarness({
      initialize: () => ({ protocol_version: '1', capabilities: [] }),
      'stats/tokens': () => pending.promise,
    })
    const obs = new VivyObservabilityController(() => true)
    await obs.attach(h.client)
    const req = obs.refresh({ period: '1d' })
    obs.detach()
    pending.resolve(snapshot())
    expect(await req).toBeNull()
    expect(obs.snapshot()).toBeNull()
  })

  it('failed refresh keeps prior totals marked stale — never blanks to zero', async () => {
    const { h, obs } = await attachConnected()
    await obs.refresh({ period: '1d' })
    expect(obs.snapshot()).not.toBeNull()
    h.setHandler('stats/tokens', () => Promise.reject(new Error('token usage store is not configured')))
    const res = await obs.refresh({ period: '3d' })
    expect(res).toBeNull()
    expect(obs.stale()).toBe(true)
    expect(obs.error()).toContain('token usage store is not configured')
    expect(obs.snapshot()?.period).toBe('1d')
  })

  it('a finished run marks retained totals stale until the next snapshot', async () => {
    const { h, obs } = await attachConnected()
    await obs.refresh({ period: '1d' })
    expect(obs.stale()).toBe(false)
    h.push({
      kind: 'vivy',
      method: 'run/event',
      params: { event: { run_id: 'r1', seq: 9, type: 'run.completed', payload: {} } },
    } as WireEvent)
    expect(obs.stale()).toBe(true)
    await obs.refresh({ period: '1d' })
    expect(obs.stale()).toBe(false)
  })

  it('request_count means usage reports — reported + legacy, not billed calls', () => {
    const snap = snapshot({
      coverage: coverage({
        state: 'partial',
        observed_calls: 5,
        completed_with_usage: 2,
        reported_calls: 2,
        legacy_usage_records: 3,
        missing_usage_calls: 2,
        active_calls: 1,
      }),
      total: {
        total_input: 1,
        total_output: 1,
        total_tokens: 2,
        total_reasoning: 0,
        total_cached: 0,
        request_count: 5,
        total_cost_usd: 0,
        cost_known: false,
      },
    })
    const view = usageView(snap)
    expect(view.reportedReports).toBe(5)
    expect(view.coverage.observed_calls).toBe(5)
  })
})
