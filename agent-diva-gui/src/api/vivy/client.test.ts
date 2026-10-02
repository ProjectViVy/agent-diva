import { describe, expect, it } from 'vitest'
import { VivyClient } from './client'
import { VivyCallError, type WireEvent } from './contracts'
import type { VivyTransport } from './transport'

function fakeTransport(
  impl: (req: { method: string; params?: unknown; timeoutMs?: number }) => Promise<unknown>,
): VivyTransport & { calls: Array<{ method: string; params?: unknown; timeoutMs?: number }> } {
  const calls: Array<{ method: string; params?: unknown; timeoutMs?: number }> = []
  return {
    calls,
    call: (req) => {
      calls.push(req)
      return impl(req)
    },
    onEvent: () => Promise.resolve(() => {}),
    close: () => {},
  }
}

describe('VivyClient transport contract', () => {
  it('forwards method and params verbatim', async () => {
    const t = fakeTransport(() => Promise.resolve({ ok: 1 }))
    const c = new VivyClient(t)
    await c.call('session/get', { session_id: 's1' })
    expect(t.calls).toEqual([{ method: 'session/get', params: { session_id: 's1' }, timeoutMs: undefined }])
  })

  it('resolves concurrent calls independently (correlation is the transport\u2019s job)', async () => {
    const t = fakeTransport((req) => Promise.resolve({ for: req.method }))
    const c = new VivyClient(t)
    const [a, b] = await Promise.all([
      c.call<{ for: string }>('run/get', { run_id: 'r1' }),
      c.call<{ for: string }>('run/get', { run_id: 'r2' }),
    ])
    expect(a.for).toBe('run/get')
    expect(b.for).toBe('run/get')
    expect(t.calls).toHaveLength(2)
  })

  it('preserves RPC error kind/code/message/data', async () => {
    const body = { kind: 'rpc', code: -32004, message: 'run is not active in this process', data: { x: 1 } }
    const t = fakeTransport(() => Promise.reject(body))
    const c = new VivyClient(t)
    const err = await c.runCancel('r1').catch((e: unknown) => e)
    expect(err).toBeInstanceOf(VivyCallError)
    const e = err as VivyCallError
    expect(e.kind).toBe('rpc')
    expect(e.code).toBe(-32004)
    expect(e.message).toBe('run is not active in this process')
    expect(e.data).toEqual({ x: 1 })
    expect(e.unknownOutcome).toBe(false)
  })

  it('surfaces incompatible_abi and closed bridge failures', async () => {
    for (const kind of ['incompatible_abi', 'closed'] as const) {
      const t = fakeTransport(() => Promise.reject({ kind, code: 0, message: kind }))
      const c = new VivyClient(t)
      const err = await c.sessionList().catch((e: unknown) => e as VivyCallError)
      expect(err.kind).toBe(kind)
    }
  })

  it('never retries a timeout after a mutation was submitted', async () => {
    const t = fakeTransport(() =>
      Promise.reject({ kind: 'timeout', code: 0, message: 'call timed out' }),
    )
    const c = new VivyClient(t)
    const err = await c.turnStart('s1', 'hi').catch((e: unknown) => e as VivyCallError)
    expect(t.calls).toHaveLength(1) // no auto-resubmit, ever
    expect(err.kind).toBe('timeout')
    expect(err.unknownOutcome).toBe(true)
  })

  it('flags transport_lost on a mutation as unknown outcome too', async () => {
    const t = fakeTransport(() =>
      Promise.reject({ kind: 'transport_lost', code: 0, message: 'lost' }),
    )
    const c = new VivyClient(t)
    const err = await c.sessionDelete('s1').catch((e: unknown) => e as VivyCallError)
    expect(err.unknownOutcome).toBe(true)
    expect(t.calls).toHaveLength(1)
  })

  it('does not flag unknown outcome for read calls or non-timeout failures', async () => {
    const t = fakeTransport(() =>
      Promise.reject({ kind: 'timeout', code: 0, message: 'call timed out' }),
    )
    const c = new VivyClient(t)
    const err = await c.runGet('r1').catch((e: unknown) => e as VivyCallError)
    expect(err.kind).toBe('timeout')
    expect(err.unknownOutcome).toBe(false)
  })

  it('typed wrappers send the frozen wire methods', async () => {
    const t = fakeTransport(() => Promise.resolve({}))
    const c = new VivyClient(t)
    await c.initialize()
    await c.sessionList()
    await c.turnStart('s1', 'go')
    await c.runSubscribe('r1')
    await c.approvalRespond('a1', 'approved', 'ok')
    expect(t.calls.map((x) => x.method)).toEqual([
      'initialize',
      'session/list',
      'turn/start',
      'run/subscribe',
      'approval/respond',
    ])
    expect(t.calls[2].params).toEqual({ session_id: 's1', text: 'go' })
    expect(t.calls[4].params).toEqual({ approval_id: 'a1', decision: 'approved', reason: 'ok' })
  })

  it('close detaches listeners without touching the host', async () => {
    let closed = 0
    const t: VivyTransport = {
      call: () => Promise.resolve({}),
      onEvent: (_h: (e: WireEvent) => void) => Promise.resolve(() => {}),
      close: () => {
        closed += 1
      },
    }
    const c = new VivyClient(t)
    c.close()
    expect(closed).toBe(1)
  })
})
