/**
 * useEventStream — SSE composable with mock and production modes.
 *
 * Mock mode is activated when `VITE_USE_MOCK_SSE=true` or when running in
 * Vite dev mode (`import.meta.env.DEV`). In mock mode `connect()` is a no-op
 * and `mockEmit()` fires registered callbacks directly, making the full
 * approval flow testable without a running gateway.
 *
 * **Production mode** creates a real `EventSource` against the gateway SSE
 * endpoint with exponential backoff reconnection (1s → 2s → 4s → 8s → 16s,
 * capped at 30s). After 5 consecutive failures an error event is emitted and
 * a console error is logged.
 */

import { ref, onUnmounted } from 'vue';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Supported SSE event types. */
export type EventType = 'notebook.new_report' | 'approval.required' | 'approval.resolved';

/** Map each EventType to the shape of data its callbacks receive. */
export interface EventDataMap {
  'notebook.new_report': { report_id: string; title: string; summary: string };
  'approval.required': { request_id: string; operation: string; risk: string; scope: string; timeout_seconds: number; created_at: string };
  'approval.resolved': { request_id: string; decision: 'approved' | 'rejected' | 'expired' };
}

/** Generic callback that receives parsed JSON data for a given event type. */
export type EventCallback<T extends EventType = EventType> = (data: EventDataMap[T]) => void;

/** Callback for stream-level error events. */
export type ErrorCallback = (error: { message: string; failures: number }) => void;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Gateway SSE endpoint (production mode). */
const GATEWAY_SSE_URL = 'http://localhost:9100/api/sse/events';

/** Initial backoff delay in milliseconds. */
const BACKOFF_INITIAL_MS = 1_000;

/** Maximum backoff delay in milliseconds. */
const BACKOFF_MAX_MS = 30_000;

/** Consecutive failures before emitting an error event. */
const MAX_FAILURES = 5;

// ---------------------------------------------------------------------------
// Module state (shared across component instances)
// ---------------------------------------------------------------------------

/** Map of event type → registered callbacks. */
const listeners = new Map<EventType, Set<EventCallback>>();

/** Stream-level error listeners. */
const errorListeners = new Set<ErrorCallback>();

/** Active EventSource instance (production mode only). */
let eventSource: EventSource | null = null;

/** Whether the composable is currently connected. */
const connected = ref(false);

/** Whether a manual disconnect was requested (prevents auto-reconnect). */
let manualDisconnect = false;

/** Current consecutive failure count. */
let failureCount = 0;

/** Current backoff delay in milliseconds. */
let backoffMs = BACKOFF_INITIAL_MS;

/** Scheduled reconnect timer handle. */
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Determine whether mock mode is active. */
function isMockMode(): boolean {
  return (
    import.meta.env.VITE_USE_MOCK_SSE === 'true' ||
    import.meta.env.DEV === true
  );
}

/**
 * Internal: dispatch a raw SSE `MessageEvent` to registered listeners.
 * Parses `event.data` as JSON and forwards typed callbacks.
 */
function dispatch<T extends EventType>(eventType: T, raw: string) {
  const cbs = listeners.get(eventType);
  if (!cbs || cbs.size === 0) return;
  let parsed: EventDataMap[T];
  try {
    parsed = JSON.parse(raw) as EventDataMap[T];
  } catch {
    console.warn(`[useEventStream] Failed to parse SSE data for "${eventType}":`, raw);
    return;
  }
  for (const cb of cbs) {
    try {
      cb(parsed);
    } catch (err) {
      console.error(`[useEventStream] Listener error for "${eventType}":`, err);
    }
  }
}

/** Emit a stream-level error to all registered error listeners. */
function emitError(message: string) {
  for (const cb of errorListeners) {
    try {
      cb({ message, failures: failureCount });
    } catch (err) {
      console.error('[useEventStream] Error listener threw:', err);
    }
  }
}

/**
 * Schedule a reconnect attempt using exponential backoff.
 * Resets backoff on successful connection.
 */
function scheduleReconnect() {
  if (manualDisconnect) return;

  failureCount++;
  const delay = backoffMs;

  if (failureCount >= MAX_FAILURES) {
    const msg = `[useEventStream] ${MAX_FAILURES} consecutive failures — SSE connection lost.`;
    console.error(msg);
    emitError(msg);
  }

  // Exponential backoff: double each time, cap at BACKOFF_MAX_MS.
  backoffMs = Math.min(backoffMs * 2, BACKOFF_MAX_MS);

  console.warn(`[useEventStream] Reconnecting in ${delay}ms (attempt ${failureCount})...`);
  reconnectTimer = setTimeout(() => {
    reconnectTimer = null;
    if (!manualDisconnect) {
      openEventSource();
    }
  }, delay);
}

/** Open the EventSource and wire up handlers. */
function openEventSource() {
  if (manualDisconnect) return;

  // Close existing instance if any.
  if (eventSource) {
    eventSource.close();
    eventSource = null;
  }

  const source = new EventSource(GATEWAY_SSE_URL);

  const eventTypes: EventType[] = [
    'notebook.new_report',
    'approval.required',
    'approval.resolved',
  ];

  for (const et of eventTypes) {
    source.addEventListener(et, (ev: MessageEvent) => {
      dispatch(et, ev.data);
    });
  }

  source.onopen = () => {
    // Connection succeeded — reset backoff state.
    failureCount = 0;
    backoffMs = BACKOFF_INITIAL_MS;
    connected.value = true;
  };

  source.onerror = () => {
    connected.value = false;
    source.close();
    eventSource = null;
    scheduleReconnect();
  };

  eventSource = source;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Composable that exposes an SSE event stream abstraction.
 *
 * @example
 * ```ts
 * const { on, connect, disconnect, mockEmit } = useEventStream();
 *
 * on('approval.required', (data) => {
 *   console.log('Approval needed:', data.operation);
 * });
 *
 * connect();
 *
 * // In tests / mock mode:
 * mockEmit('approval.required', {
 *   request_id: '1',
 *   operation: 'Allow this?',
 *   risk: 'low',
 *   scope: 'file',
 *   timeout_seconds: 300,
 *   created_at: new Date().toISOString(),
 * });
 * ```
 */
export function useEventStream() {
  /**
   * Register a callback for a specific event type.
   * Returns an unsubscribe function for convenience.
   */
  function on<T extends EventType>(eventType: T, callback: EventCallback<T>): () => void {
    if (!listeners.has(eventType)) {
      listeners.set(eventType, new Set());
    }
    listeners.get(eventType)!.add(callback as EventCallback);
    return () => {
      listeners.get(eventType)?.delete(callback as EventCallback);
    };
  }

  /**
   * Register a callback for stream-level errors (e.g. connection failures).
   * Returns an unsubscribe function.
   */
  function onError(callback: ErrorCallback): () => void {
    errorListeners.add(callback);
    return () => {
      errorListeners.delete(callback);
    };
  }

  /**
   * Start listening for SSE events.
   *
   * - **Mock mode**: no-op (events are injected via `mockEmit`).
   * - **Production mode**: opens an `EventSource` to the gateway SSE endpoint
   *   with exponential backoff reconnection.
   */
  function connect(): void {
    if (connected.value) return;
    manualDisconnect = false;

    if (isMockMode()) {
      // Mock mode — nothing to connect; events come from mockEmit().
      connected.value = true;
      return;
    }

    // --- Production mode ---------------------------------------------------
    openEventSource();
  }

  /**
   * Disconnect the SSE stream, cancel pending reconnects, and remove all
   * registered listeners. Safe to call even when not connected.
   */
  function disconnect(): void {
    manualDisconnect = true;

    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = null;
    }

    if (eventSource) {
      eventSource.close();
      eventSource = null;
    }

    // Reset backoff state for next connect() call.
    failureCount = 0;
    backoffMs = BACKOFF_INITIAL_MS;

    listeners.clear();
    errorListeners.clear();
    connected.value = false;
  }

  /**
   * Mock-only: manually emit an event as if it came from the gateway.
   *
   * This fires all registered callbacks for the given event type immediately.
   * Intended for development and testing when mock mode is active.
   *
   * @param eventType - The event type to emit.
   * @param data      - The parsed data payload to deliver to listeners.
   */
  function mockEmit<T extends EventType>(eventType: T, data: EventDataMap[T]): void {
    if (!isMockMode()) {
      console.warn('[useEventStream] mockEmit() called outside mock mode — ignoring.');
      return;
    }
    const cbs = listeners.get(eventType);
    if (!cbs || cbs.size === 0) return;
    for (const cb of cbs) {
      try {
        cb(data);
      } catch (err) {
        console.error(`[useEventStream] Listener error for "${eventType}":`, err);
      }
    }
  }

  // Auto-cleanup when the owning component unmounts.
  onUnmounted(() => {
    disconnect();
  });

  return {
    /** Whether the stream is currently connected. */
    connected,
    /** Register a typed callback for an event type. Returns an unsubscribe fn. */
    on,
    /** Register a callback for stream-level errors. Returns an unsubscribe fn. */
    onError,
    /** Open the SSE connection (no-op in mock mode). */
    connect,
    /** Close the connection and remove all listeners. */
    disconnect,
    /** Mock-only: inject an event to test listeners. */
    mockEmit,
  };
}
