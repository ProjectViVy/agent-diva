/**
 * Transport seam (DN-1). This is the ONLY module allowed to touch the
 * native host for VIVY traffic; every business-facing call goes through
 * `VivyTransport.call` and every backend event through
 * `VivyTransport.onEvent`. Presentation/native calls unrelated to VIVY
 * keep using `src/platform/desktop-host.ts` directly.
 */
import { onVivyEvent, vivyCall, type VivyCallRequest, type WireEvent } from '../../platform/desktop-host'

export interface VivyTransport {
  // Wails transport: `call` -> bound RuntimeService.VivyCall (CallReply
  // envelope), `onEvent` -> one Events.On('vivy:event') listener.

  call(request: VivyCallRequest): Promise<unknown>
  /**
   * Subscribe to bridge events; returns an unsubscribe function.
   * A call after `close()` must leave existing listeners detached and
   * resolve to a no-op unsubscriber.
   */
  onEvent(handler: (event: WireEvent) => void): Promise<() => void>
  /** Detaches frontend listeners only — never tears the host down. */
  close(): void
}

export function createWailsTransport(): VivyTransport {
  const handlers = new Set<(event: WireEvent) => void>()
  let unlisten: (() => void) | null = null
  let installation: Promise<void> | null = null
  let closed = false

  const dispatch = (event: WireEvent) => {
    for (const handler of handlers) handler(event)
  }

  const ensureListener = (): Promise<void> => {
    if (closed || unlisten !== null) return Promise.resolve()
    if (installation !== null) return installation

    let attempt: Promise<void>
    attempt = Promise.resolve()
      .then(() => closed ? null : onVivyEvent(dispatch))
      .then((detach) => {
        if (detach === null) return
        if (closed) detach()
        else unlisten = detach
      })
      .finally(() => {
        if (installation === attempt) installation = null
      })
    installation = attempt
    return attempt
  }

  return {
    call(request) {
      return vivyCall(request)
    },
    async onEvent(handler) {
      if (closed) return () => {}
      handlers.add(handler)
      try {
        await ensureListener()
      } catch (error) {
        handlers.delete(handler)
        throw error
      }
      if (closed) {
        handlers.delete(handler)
        return () => {}
      }
      let subscribed = true
      return () => {
        if (!subscribed) return
        subscribed = false
        handlers.delete(handler)
      }
    },
    close() {
      if (closed) return
      closed = true
      handlers.clear()
      const detach = unlisten
      unlisten = null
      detach?.()
    },
  }
}
