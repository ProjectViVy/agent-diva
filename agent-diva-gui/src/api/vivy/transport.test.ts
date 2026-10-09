import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { WireEvent } from '../../platform/desktop-host'
import type { UnlistenFn } from '../../platform/desktop-host'

const { onVivyEvent } = vi.hoisted(() => ({ onVivyEvent: vi.fn() }))

vi.mock('../../platform/desktop-host', () => ({
  onVivyEvent,
  vivyCall: vi.fn(),
}))

import { createWailsTransport } from './transport'

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason: unknown) => void
  const promise = new Promise<T>((res, rej) => {
    resolve = res
    reject = rej
  })
  return { promise, resolve, reject }
}

describe('Wails VIVY event transport', () => {
  beforeEach(() => onVivyEvent.mockReset())

  it('shares one pending native listener across concurrent subscribers', async () => {
    const pending = deferred<UnlistenFn>()
    onVivyEvent.mockReturnValue(pending.promise)
    const transport = createWailsTransport()
    const first = vi.fn()
    const second = vi.fn()

    const firstOff = transport.onEvent(first)
    const secondOff = transport.onEvent(second)
    await Promise.resolve()
    const installCalls = onVivyEvent.mock.calls.length

    const event: WireEvent = { kind: 'vivy', method: 'run.delta', params: { n: 1 } }
    const dispatch = onVivyEvent.mock.calls[0][0] as (event: WireEvent) => void
    pending.resolve(vi.fn())
    const [unsubscribeFirst, unsubscribeSecond] = await Promise.all([firstOff, secondOff])
    expect(installCalls).toBe(1)
    dispatch(event)
    expect(first).toHaveBeenCalledExactlyOnceWith(event)
    expect(second).toHaveBeenCalledExactlyOnceWith(event)

    unsubscribeFirst()
    dispatch(event)
    expect(first).toHaveBeenCalledTimes(1)
    expect(second).toHaveBeenCalledTimes(2)
    unsubscribeSecond()
    transport.close()
  })

  it('detaches exactly once when closed while installation is pending', async () => {
    const pending = deferred<UnlistenFn>()
    onVivyEvent.mockReturnValue(pending.promise)
    const transport = createWailsTransport()
    const unsubscribe = transport.onEvent(vi.fn())
    await Promise.resolve()
    expect(onVivyEvent).toHaveBeenCalledTimes(1)

    transport.close()
    transport.close()
    const nativeOff = vi.fn()
    pending.resolve(nativeOff)
    expect(await unsubscribe).toBeTypeOf('function')
    expect(nativeOff).toHaveBeenCalledTimes(1)
    expect(await transport.onEvent(vi.fn())).toBeTypeOf('function')
    expect(onVivyEvent).toHaveBeenCalledTimes(1)
  })

  it('closes an installed listener idempotently and rejects post-close installs', async () => {
    const nativeOff = vi.fn()
    onVivyEvent.mockResolvedValue(nativeOff)
    const transport = createWailsTransport()
    await transport.onEvent(vi.fn())

    transport.close()
    transport.close()
    expect(nativeOff).toHaveBeenCalledTimes(1)
    await transport.onEvent(vi.fn())
    expect(onVivyEvent).toHaveBeenCalledTimes(1)
  })

  it('rejects all subscribers on a failed shared install and retries cleanly', async () => {
    const pending = deferred<UnlistenFn>()
    const installError = new Error('listener install failed')
    const retryOff = vi.fn()
    onVivyEvent.mockReturnValueOnce(pending.promise).mockResolvedValueOnce(retryOff)
    const transport = createWailsTransport()
    const firstHandler = vi.fn()
    const secondHandler = vi.fn()
    const first = transport.onEvent(firstHandler)
    const second = transport.onEvent(secondHandler)
    await Promise.resolve()
    const installCalls = onVivyEvent.mock.calls.length

    pending.reject(installError)
    const outcomes = await Promise.allSettled([first, second])
    expect(installCalls).toBe(1)
    expect(outcomes).toEqual([
      { status: 'rejected', reason: installError },
      { status: 'rejected', reason: installError },
    ])

    const retry = await transport.onEvent(vi.fn())
    expect(onVivyEvent).toHaveBeenCalledTimes(2)
    const dispatch = onVivyEvent.mock.calls[1][0] as (event: WireEvent) => void
    dispatch({ kind: 'bridge', status: 'gap' })
    expect(firstHandler).not.toHaveBeenCalled()
    expect(secondHandler).not.toHaveBeenCalled()
    retry()
    transport.close()
    expect(retryOff).toHaveBeenCalledTimes(1)
  })
})
