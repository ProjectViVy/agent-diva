/**
 * DN-4D — cognitive action API tests.
 *
 * Replays the DN-4C captured fixture `closure-cognitive-actions.json`: every
 * `result` object was produced by the real `module.action.invoke` path on the
 * pinned VIVY backend (cognitive_origin_test.go), so parsing must accept them
 * verbatim and preserve domain codes.
 */
import { describe, expect, it, vi } from 'vitest';
import fixture from '../../../docs/plans/diva-next/fixtures/closure-cognitive-actions.json';
import { createCognitiveClient, CognitiveProtocolError } from './cognitive';
import type { VivyClient } from './vivy/client';
import { VivyCallError } from './vivy/contracts';

interface FixtureAction {
  action_id: string;
  input: Record<string, unknown>;
  result: unknown;
}

const actions = fixture.actions as FixtureAction[];

function clientReturning(results: unknown[]) {
  const call = vi.fn();
  for (const r of results) call.mockResolvedValueOnce(r);
  return { call } as unknown as VivyClient & { call: ReturnType<typeof vi.fn> };
}

function suffixOf(actionId: string) {
  return actionId.replace('diva.cognitive.', '');
}

describe('cognitive api — captured fixture replay', () => {
  it('replays every captured action envelope verbatim', async () => {
    for (const a of actions) {
      const client = clientReturning([a.result]);
      const api = createCognitiveClient(client);
      const out = await api.call(suffixOf(a.action_id) as never, a.input);
      expect(out).toEqual(a.result);
    }
    expect(actions).toHaveLength(20);
  });

  it('routes through module.action.invoke with the sealed module id and action prefix', async () => {
    const client = clientReturning([{ status: 'ok', value: { items: [] } }]);
    const api = createCognitiveClient(client);
    await api.call('persona.reviews.list', { session_id: 'sess_x' });
    expect(client.call).toHaveBeenCalledWith(
      'module.action.invoke',
      {
        module_id: 'vivy/diva-cognitive',
        action_id: 'diva.cognitive.persona.reviews.list',
        input: { session_id: 'sess_x' },
      },
      { mutation: false },
    );
  });

  it('marks writes as mutations and reads as non-mutations', async () => {
    const ok = { status: 'ok', value: null };
    const client = clientReturning(Array(20).fill(ok));
    const api = createCognitiveClient(client);
    const writes = [
      'persona.initialize',
      'persona.save',
      'persona.review.decide',
      'actmem.work.patch',
      'actmem.owner.save',
      'memory.mutate',
      'policy.set',
      'trigger',
      'cancel',
    ];
    const reads = [
      'status',
      'persona.read',
      'persona.reviews.list',
      'frozen.read',
      'actmem.read',
      'actmem.owner.read',
      'memory.search',
      'memory.expand',
      'memory.receipt',
      'policy.get',
      'results.list',
    ];
    for (const w of writes) await api.call(w as never, { session_id: 's' });
    for (const r of reads) await api.call(r as never, { session_id: 's' });
    const flags = client.call.mock.calls.map((c) => (c[2] as { mutation: boolean }).mutation);
    expect(flags.slice(0, writes.length)).toEqual(writes.map(() => true));
    expect(flags.slice(writes.length)).toEqual(reads.map(() => false));
  });

  it('maps a mutation timeout into a typed unknown outcome, never an automatic replay', async () => {
    const client = {
      call: vi.fn().mockRejectedValue(
        new VivyCallError({ kind: 'timeout', code: 0, message: 'call timed out' }, true),
      ),
    } as unknown as VivyClient;
    const api = createCognitiveClient(client);
    const out = await api.call('persona.save', {
      session_id: 's',
      kind: 'identity',
      content: 'c',
      base_revision: 1,
      reason: 'r',
    });
    expect(out.status).toBe('unknown');
    if (out.status !== 'ok') {
      expect(out.error.code).toBe('outcome_unknown');
      expect(out.error.retryable).toBe(false);
    }
    expect(client.call).toHaveBeenCalledTimes(1);
  });

  it('rethrows transport denials (rpc errors are host-owned, not business outcomes)', async () => {
    const denial = fixture.denials[0];
    const client = {
      call: vi.fn().mockRejectedValue(
        new VivyCallError({ kind: 'rpc', code: -32009, message: 'module action is not authorized' }),
      ),
    } as unknown as VivyClient;
    const api = createCognitiveClient(client);
    await expect(api.call('status', { session_id: 'sess_nonexistent' })).rejects.toMatchObject({
      code: -32009,
    });
    expect(denial.error).toContain('-32009');
  });

  it('rejects malformed envelopes instead of inventing an outcome', async () => {
    for (const bad of [null, 42, 'ok', { status: 'weird' }, { status: 'failed' }, { status: 'ok' }]) {
      const client = clientReturning([bad]);
      const api = createCognitiveClient(client);
      await expect(api.call('status', { session_id: 's' })).rejects.toBeInstanceOf(
        CognitiveProtocolError,
      );
    }
  });

  it('rejects unsafe numeric input fields before any transport call', async () => {
    const client = clientReturning([]);
    const api = createCognitiveClient(client);
    for (const input of [
      { session_id: 's', base_revision: 1.5 },
      { session_id: 's', limit: Number.MAX_SAFE_INTEGER + 1 },
      { session_id: 's', min_interval_ms: Number.NaN },
      { session_id: 's', expected_revision: Number.POSITIVE_INFINITY },
    ]) {
      await expect(api.call('persona.save', input as never)).rejects.toBeInstanceOf(
        CognitiveProtocolError,
      );
    }
    expect(client.call).not.toHaveBeenCalled();
  });

  it('requires a non-empty session_id on every action', async () => {
    const client = clientReturning([]);
    const api = createCognitiveClient(client);
    await expect(api.call('status', { session_id: '' })).rejects.toBeInstanceOf(
      CognitiveProtocolError,
    );
    await expect(api.call('status', {} as never)).rejects.toBeInstanceOf(CognitiveProtocolError);
    expect(client.call).not.toHaveBeenCalled();
  });

  it('preserves failed envelopes carrying recovery payloads (actmem.owner.save)', async () => {
    const a = actions.find((x) => x.action_id === 'diva.cognitive.actmem.owner.save')!;
    const client = clientReturning([a.result]);
    const api = createCognitiveClient(client);
    const out = await api.call('actmem.owner.save', a.input);
    expect(out.status).toBe('failed');
    if (out.status !== 'ok') {
      expect(out.error.code).toBe('actmem_rejected');
      expect((out.value as { revision: number }).revision).toBe(0);
    }
  });
});
