import { describe, expect, it, vi } from 'vitest';

vi.mock('./vivy/instance', () => ({
  vivyClient: { call: vi.fn() },
}));

import { vivyClient } from './vivy/instance';
import {
  createMask,
  deleteMask,
  getMaskSelection,
  listAllMasks,
  listMasks,
  MaskActionError,
  setMaskSelection,
  updateMask,
} from './masks';

// NOTE: no beforeEach mockClear/mockReset — clearing the mocked call between
// tests makes vitest 4 report mock-rejections as unhandled test errors.
const call = vivyClient.call as unknown as {
  (method: string, params?: unknown, opts?: unknown): Promise<unknown>;
  mock: { calls: [string, any, any][] };
  mockResolvedValue(v: unknown): unknown;
  mockRejectedValue(v: unknown): unknown;
  mockResolvedValueOnce(v: unknown): unknown;
};

const lastParams = () => call.mock.calls.at(-1)![1];
const callCount = () => call.mock.calls.length;

const definition = {
  id: 'mask-1',
  name: 'Coder',
  description: 'd',
  body: 'b',
  revision: 3,
  digest: 'dg',
  built_in: false,
  generation_id: 'g1',
};

describe('masks api — module.action.invoke surface', () => {
  it('routes every call through vivy/masks with verbatim params', async () => {
    call.mockResolvedValue({ items: [definition], next_after_id: '' });
    await listMasks({ afterId: 'mask-0', limit: 25 });
    expect(call.mock.calls.at(-1)).toEqual([
      'module.action.invoke',
      {
        module_id: 'vivy/masks',
        action_id: 'vivy.masks.catalog.list',
        input: { after_id: 'mask-0', limit: 25 },
      },
      { mutation: true },
    ]);
  });

  it('paginates catalog.list until next_after_id is empty', async () => {
    const before = callCount();
    call
      .mockResolvedValueOnce({ items: [{ ...definition, id: 'a' }], next_after_id: 'a' })
      .mockResolvedValueOnce({ items: [{ ...definition, id: 'b' }], next_after_id: '' });
    const all = await listAllMasks();
    expect(all.map((m) => m.id)).toEqual(['a', 'b']);
    expect(callCount() - before).toBe(2);
  });

  it('stops pagination when a page returns no items (defensive)', async () => {
    const before = callCount();
    call.mockResolvedValueOnce({ items: [], next_after_id: 'loop' });
    expect(await listAllMasks()).toEqual([]);
    expect(callCount() - before).toBe(1);
  });

  it('sends operation_id UUID on create', async () => {
    call.mockResolvedValue(definition);
    await createMask({ name: 'n', description: 'd', body: 'b' });
    const input = lastParams().input;
    expect(input.operation_id).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i,
    );
    expect(input.name).toBe('n');
  });

  it('sends expected_revision on update/delete/select', async () => {
    call.mockResolvedValue(definition);
    await updateMask({ id: 'm', expectedRevision: 4, name: 'n', description: 'd', body: 'b' });
    expect(lastParams().input).toEqual({
      id: 'm', expected_revision: 4, name: 'n', description: 'd', body: 'b',
    });

    call.mockResolvedValue({ deleted: true });
    await deleteMask('m', 7);
    expect(lastParams().input).toEqual({ id: 'm', expected_revision: 7 });

    call.mockResolvedValue({ session_id: 's1', mask_id: 'm', revision: 9, available: true, inactive_reason: '' });
    await setMaskSelection('s1', 'm', 8);
    expect(lastParams().input).toEqual({
      session_id: 's1', mask_id: 'm', expected_revision: 8,
    });
  });

  it('selection.get is scoped to session_id', async () => {
    call.mockResolvedValue({ session_id: 's1', mask_id: 'm', revision: 1, available: true, inactive_reason: '' });
    const sel = await getMaskSelection('s1');
    expect(sel.mask_id).toBe('m');
    expect(lastParams().action_id).toBe('vivy.masks.selection.get');
  });

  it('surfaces wire error.data.code as MaskActionError', async () => {
    call.mockRejectedValue({
      kind: 'backend',
      code: -32009,
      message: 'mask action failed',
      data: { code: 'revision_conflict', current_revision: 12 },
    });
    const err = await updateMask({ id: 'm', expectedRevision: 4, name: 'n', description: 'd', body: 'b' })
      .catch((e) => e);
    expect(err).toBeInstanceOf(MaskActionError);
    expect(err.code).toBe('revision_conflict');
    expect(err.currentRevision).toBe(12);
  });

  it('mask_in_use carries reference_count', async () => {
    call.mockRejectedValue({
      kind: 'backend', code: -32009, message: 'mask action failed',
      data: { code: 'mask_in_use', reference_count: 3 },
    });
    const err = await deleteMask('m', 1).catch((e) => e);
    expect(err.code).toBe('mask_in_use');
    expect(err.referenceCount).toBe(3);
  });

  it('untyped backend errors still propagate as MaskActionError without code', async () => {
    call.mockRejectedValue({ kind: 'backend', code: -32601, message: 'module action capability is not configured' });
    const err = await listMasks().catch((e) => e);
    expect(err).toBeInstanceOf(MaskActionError);
    expect(err.code).toBeNull();
  });
});
