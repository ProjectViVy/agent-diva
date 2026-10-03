import { beforeEach, describe, expect, it, vi } from 'vitest';

const statsTokens = vi.fn();
vi.mock('./vivy/instance', () => ({
  vivyClient: { statsTokens: (params: unknown) => statsTokens(params) },
}));

import { getTokenStats } from './tokenStats';

beforeEach(() => {
  statsTokens.mockReset();
});

describe('token statistics API', () => {
  it('returns the stats/tokens snapshot for the requested period', async () => {
    const snapshot = {
      period: '1w',
      scope: 'chat_runs',
      projection_version: 2,
      coverage: {
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
      },
      total: {
        total_input: 12,
        total_output: 8,
        total_tokens: 20,
        total_reasoning: 0,
        total_cached: 0,
        request_count: 1,
        total_cost_usd: 0,
        cost_known: false,
      },
      models: [],
      providers: [],
      timeline: [],
      sessions: [],
    };
    statsTokens.mockResolvedValueOnce(snapshot);

    await expect(getTokenStats('1w')).resolves.toEqual(snapshot);
    expect(statsTokens).toHaveBeenCalledWith({
      period: '1w',
      tz_offset_minutes: expect.any(Number),
      session_limit: 20,
    });
  });

  it('propagates backend failures for the panel fallback path', async () => {
    statsTokens.mockRejectedValueOnce(new Error('token usage store is not configured'));
    await expect(getTokenStats()).rejects.toThrow('token usage store is not configured');
  });
});
