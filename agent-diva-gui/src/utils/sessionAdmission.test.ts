import { describe, expect, it } from 'vitest';
import { routeSessionAdmission, type SessionAdmissionPayload } from './sessionAdmission';

function payload(requestId: string, phase: string, code: string | null = null): SessionAdmissionPayload {
  return {
    request_id: requestId,
    data: {
      code,
      phase,
      session_key: 'gui:chat',
      request_id: requestId,
      trace_id: `trace-${requestId}`,
      queue_depth: 2,
      wait_latency_ms: 12,
    },
  };
}

describe('routeSessionAdmission', () => {
  it('routes queued and running transitions for the active request', () => {
    expect(routeSessionAdmission('request-a', payload('request-a', 'queued'))).toEqual({
      kind: 'queued',
      depth: 2,
      waitLatencyMs: 12,
    });
    expect(routeSessionAdmission('request-a', payload('request-a', 'running'))).toEqual({
      kind: 'running',
    });
  });

  it('routes known terminal codes', () => {
    for (const code of [
      'session_queue_full',
      'session_queue_wait_timeout',
      'session_reset',
      'session_worker_unavailable',
      'session_turn_cancelled',
    ]) {
      expect(routeSessionAdmission('request-a', payload('request-a', 'rejected', code))).toEqual({
        kind: 'terminal',
        code,
      });
    }
  });

  it('ignores stale wrapper or observation request ids', () => {
    expect(routeSessionAdmission('request-new', payload('request-old', 'queued'))).toEqual({
      kind: 'ignore',
    });
    const mismatched = payload('request-new', 'queued');
    mismatched.data.request_id = 'request-old';
    expect(routeSessionAdmission('request-new', mismatched)).toEqual({ kind: 'ignore' });
  });
});
