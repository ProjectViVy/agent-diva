/**
 * Transport seam (DN-1). This is the ONLY module allowed to touch the
 * native host for VIVY traffic; every business-facing call goes through
 * `VivyTransport.call` and every backend event through
 * `VivyTransport.onEvent`. Presentation/native calls unrelated to VIVY
 * keep using `src/platform/desktop-host.ts` directly.
 */
import { onVivyEvent, vivyCall, type VivyCallRequest, type WireEvent } from '../../platform/desktop-host'

export interface VivyTransport {
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

export function createTauriTransport(): VivyTransport {
  const handlers = new Set<(event: WireEvent) => void>()
  let unlisten: (() => void) | null = null
  let closed = false

  const ensureListener = () => {
    if (unlisten !== null || closed) return
    void onVivyEvent((event) => {
      for (const handler of handlers) handler(event)
    }).then((fn) => {
      if (closed) {
        fn()
        return
      }
      unlisten = fn
    })
  }

  return {
    call(request) {
      return vivyCall(request)
    },
    async onEvent(handler) {
      if (closed) return () => {}
      handlers.add(handler)
      ensureListener()
      return () => {
        handlers.delete(handler)
      }
    },
    close() {
      closed = true
      handlers.clear()
      unlisten?.()
      unlisten = null
    },
  }
}
