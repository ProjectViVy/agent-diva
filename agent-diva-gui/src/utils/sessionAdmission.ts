export interface SessionAdmissionObservation {
  code?: string | null;
  phase: string;
  session_key: string;
  request_id: string;
  trace_id: string;
  queue_depth: number;
  wait_latency_ms: number;
}

export interface SessionAdmissionPayload {
  request_id: string;
  data: SessionAdmissionObservation;
}

export type SessionAdmissionAction =
  | { kind: 'queued'; depth: number; waitLatencyMs: number }
  | { kind: 'running' }
  | { kind: 'terminal'; code: string }
  | { kind: 'ignore' };

const TERMINAL_CODES = new Set([
  'session_queue_full',
  'session_queue_wait_timeout',
  'session_reset',
  'session_worker_unavailable',
  'session_turn_cancelled',
]);

export function routeSessionAdmission(
  activeRequestId: string | null,
  payload: SessionAdmissionPayload,
): SessionAdmissionAction {
  if (
    !activeRequestId
    || payload.request_id !== activeRequestId
    || payload.data.request_id !== activeRequestId
  ) {
    return { kind: 'ignore' };
  }
  if (payload.data.phase === 'queued') {
    return {
      kind: 'queued',
      depth: payload.data.queue_depth,
      waitLatencyMs: payload.data.wait_latency_ms,
    };
  }
  if (payload.data.phase === 'running') return { kind: 'running' };
  if (payload.data.code && TERMINAL_CODES.has(payload.data.code)) {
    return { kind: 'terminal', code: payload.data.code };
  }
  return { kind: 'ignore' };
}
