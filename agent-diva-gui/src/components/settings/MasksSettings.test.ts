import { describe, expect, it, vi } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';

vi.mock('../../api/desktop', () => ({
  isTauriRuntime: () => true,
}));

const api = vi.hoisted(() => ({
  listAllMasks: vi.fn(),
  getMask: vi.fn(),
  getMaskSelection: vi.fn(),
  setMaskSelection: vi.fn(),
  createMask: vi.fn(),
  updateMask: vi.fn(),
  deleteMask: vi.fn(),
  MaskActionError: class MaskActionError extends Error {
    constructor(
      message: string,
      readonly code: string | null,
      readonly currentRevision?: number,
      readonly referenceCount?: number,
    ) { super(message); this.name = 'MaskActionError'; }
  },
}));

vi.mock('../../api/masks', () => api);

import MasksSettings from './MasksSettings.vue';
import { MaskActionError } from '../../api/masks';

const i18n = createI18n({ legacy: false, locale: 'en', messages: { en: {} } });

// No mockClear/mockReset — clearing vitest-4 mocks that carry rejections is
// reported as an unhandled test error. Stubs are reset per-test and call
// records are truncated directly instead.
function resetCalls() {
  for (const fn of Object.values(api)) {
    if (typeof fn === 'function' && 'mock' in fn) {
      (fn as ReturnType<typeof vi.fn>).mock.calls.length = 0;
    }
  }
}

const mask = (id: string, builtIn = false, revision = 1) => ({
  id, name: `M-${id}`, description: 'd', body: 'b',
  revision, digest: 'x', built_in: builtIn, generation_id: 'g',
});
const sel = (maskId = 'm1', revision = 2) => ({
  session_id: 's1', mask_id: maskId, revision, available: true, inactive_reason: '',
});

function mountView(sessionKey = 's1') {
  return mount(MasksSettings, {
    props: { currentSessionKey: sessionKey },
    global: { plugins: [i18n] },
  });
}

describe('MasksSettings', () => {
  it('loads catalog + session selection on mount', async () => {
    api.listAllMasks.mockResolvedValue([mask('m1'), mask('m2')]);
    api.getMaskSelection.mockResolvedValue(sel());
    resetCalls();
    const w = mountView();
    await flushPromises();
    expect(api.listAllMasks).toHaveBeenCalled();
    expect(api.getMaskSelection).toHaveBeenCalledWith('s1');
    expect(w.findAll('.masks-item')).toHaveLength(2);
  });

  it('skips selection when there is no active session', async () => {
    api.listAllMasks.mockResolvedValue([mask('m1')]);
    resetCalls();
    const w = mountView('');
    await flushPromises();
    expect(api.getMaskSelection).not.toHaveBeenCalled();
    expect(w.find('.masks-item-main').attributes('disabled')).toBeDefined();
  });

  it('marks the bound mask active and selects another with expected_revision', async () => {
    api.listAllMasks.mockResolvedValue([mask('m1'), mask('m2')]);
    api.getMaskSelection.mockResolvedValue(sel('m1', 2));
    api.setMaskSelection.mockResolvedValue(sel('m2', 3));
    resetCalls();
    const w = mountView();
    await flushPromises();
    expect(w.findAll('.masks-item')[0].classes()).toContain('active');

    await w.findAll('.masks-item-main')[1].trigger('click');
    await flushPromises();
    expect(api.setMaskSelection).toHaveBeenCalledWith('s1', 'm2', 2);
    expect(w.findAll('.masks-item')[1].classes()).toContain('active');
  });

  it('shows conflict message and resyncs selection on revision_conflict', async () => {
    api.listAllMasks.mockResolvedValue([mask('m1')]);
    api.getMaskSelection.mockResolvedValue(sel('m1', 2));
    api.setMaskSelection.mockRejectedValue(
      new MaskActionError('mask action failed', 'revision_conflict', 5),
    );
    resetCalls();
    const w = mountView();
    await flushPromises();
    await w.find('.masks-item-main').trigger('click');
    await flushPromises();
    expect(w.find('.settings-error').exists()).toBe(true);
    expect(api.getMaskSelection).toHaveBeenCalledTimes(2);
    api.setMaskSelection.mockResolvedValue(sel());
  });

  it('update sends expected_revision; conflict discards draft and refreshes', async () => {
    api.listAllMasks.mockResolvedValue([mask('m1', false, 4)]);
    api.getMaskSelection.mockResolvedValue(sel());
    api.getMask.mockResolvedValue(mask('m1', false, 4));
    api.updateMask.mockRejectedValue(new MaskActionError('mask action failed', 'revision_conflict'));
    resetCalls();
    const w = mountView();
    await flushPromises();
    await w.find('.masks-item-actions .settings-icon-btn').trigger('click');
    await flushPromises();
    expect(api.getMask).toHaveBeenCalledWith('m1');
    const draft = w.find('.masks-editor');
    expect(draft.exists()).toBe(true);
    await draft.trigger('submit');
    await flushPromises();
    expect(api.updateMask).toHaveBeenCalledWith(expect.objectContaining({
      id: 'm1', expectedRevision: 4,
    }));
    expect(w.find('.masks-editor').exists()).toBe(false); // draft discarded
    expect(w.find('.settings-error').exists()).toBe(true);
    api.updateMask.mockResolvedValue(mask('m1'));
  });

  it('built-in masks render without delete affordance', async () => {
    api.listAllMasks.mockResolvedValue([mask('b1', true)]);
    api.getMaskSelection.mockResolvedValue(sel());
    resetCalls();
    const w = mountView();
    await flushPromises();
    expect(w.findAll('.masks-item-actions button')).toHaveLength(1); // edit only
  });

  it('shows unavailable state when the capability is not compiled', async () => {
    api.listAllMasks.mockRejectedValue(new MaskActionError('module action capability is not configured', null));
    resetCalls();
    const w = mountView();
    await flushPromises();
    expect(w.find('.settings-empty').exists()).toBe(true);
    api.listAllMasks.mockResolvedValue([]);
  });
});
