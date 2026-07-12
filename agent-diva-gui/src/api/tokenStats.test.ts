import { beforeEach, describe, expect, it, vi } from 'vitest';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

import { getTokenUsageModels, getTokenUsageTotal } from './tokenStats';

beforeEach(() => {
  invoke.mockReset();
});

describe('token statistics API', () => {
  it('uses the direct DTO returned by Tauri commands', async () => {
    const total = {
      total_input: 12,
      total_output: 8,
      total_tokens: 20,
      total_cache_creation: 0,
      total_cache_read: 0,
      request_count: 1,
      total_cost: 0,
    };
    invoke.mockResolvedValueOnce(total).mockResolvedValueOnce([]);

    await expect(getTokenUsageTotal('1w')).resolves.toEqual(total);
    await expect(getTokenUsageModels('1w')).resolves.toEqual([]);
    expect(invoke).toHaveBeenNthCalledWith(1, 'get_token_usage_total', { period: '1w' });
    expect(invoke).toHaveBeenNthCalledWith(2, 'get_token_usage_models', { period: '1w' });
  });

  it('preserves Tauri command failures for the panel fallback path', async () => {
    invoke.mockRejectedValueOnce(new Error('gateway unavailable'));
    await expect(getTokenUsageTotal()).rejects.toThrow('gateway unavailable');
  });
});
