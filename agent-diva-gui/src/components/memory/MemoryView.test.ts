import { describe, it, expect, vi, beforeEach } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import MemoryView from './MemoryView.vue';
import * as desktop from '../../api/desktop';
import * as appToast from '../../utils/appToast';
import * as appDialog from '../../utils/appDialog';
import en from '../../locales/en';

vi.mock('../../api/desktop', async () => {
  const actual = await vi.importActual<typeof desktop>('../../api/desktop');
  return {
    ...actual,
    bmlListMemories: vi.fn(),
    bmlGetMemory: vi.fn(),
    bmlRemoveMemory: vi.fn(),
  };
});

vi.mock('../../utils/appToast', async () => {
  const actual = await vi.importActual<typeof appToast>('../../utils/appToast');
  return {
    ...actual,
    showAppToast: vi.fn(),
  };
});

const appPrompt = vi.fn<() => Promise<string | null>>(() => Promise.resolve('stale fact'));
vi.mock('../../utils/appDialog', () => ({
  appPrompt: (...args: unknown[]) => appPrompt(...args),
}));

const i18n = createI18n({
  legacy: false,
  locale: 'en',
  messages: { en },
});

function makeMemory(overrides: Partial<desktop.BmlMemory> = {}): desktop.BmlMemory {
  return {
    id: 'mem-1',
    kind: 'long_term',
    content: 'kestrel patrols the ridge at dawn',
    provenance: {
      source: 'auto_dream',
      source_id: 'wave4-run',
      content_digest: { algorithm: 'sha256', value: 'abc' },
      captured_at: '2026-07-05T12:00:00Z',
      correlation: {
        request_id: 'req-1',
        turn_id: 'turn-1',
        session_id: 'session-1',
        trace_id: null,
      },
    },
    evidence_refs: [],
    confidence_bps: 8000,
    sensitivity: 'internal',
    trust: 'applied_authority',
    scope: { tenant_id: 'local', workspace_id: 'ws-1', session_id: null },
    created_at: '2026-07-05T12:00:00Z',
    effective_at: '2026-07-05T12:00:00Z',
    expires_at: null,
    supersedes: [],
    tombstone: null,
    ...overrides,
  };
}

function makeStored(overrides: Partial<desktop.BmlMemory> = {}): desktop.BmlStoredMemory {
  return { record: makeMemory(overrides), revision: 3 };
}

function factory() {
  return mount(MemoryView, {
    global: {
      plugins: [i18n],
    },
  });
}

describe('MemoryView', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    appPrompt.mockReset();
    appPrompt.mockResolvedValue('stale fact');
    (desktop.bmlListMemories as ReturnType<typeof vi.fn>).mockResolvedValue({
      status: 'ok',
      memories: [makeStored()],
    });
    (desktop.bmlGetMemory as ReturnType<typeof vi.fn>).mockImplementation((id: string) =>
      Promise.resolve({ status: 'ok', memory: makeStored({ id }) }),
    );
  });

  it('renders the memory list after loading', async () => {
    const wrapper = factory();
    await flushPromises();
    expect(wrapper.text()).toContain('kestrel patrols the ridge at dawn');
    expect(wrapper.text()).toContain('Long term');
  });

  it('shows detail when a memory is selected', async () => {
    const wrapper = factory();
    await flushPromises();
    await wrapper.find('.memory-list-item').trigger('click');
    await flushPromises();
    expect(desktop.bmlGetMemory).toHaveBeenCalledWith('mem-1');
    expect(wrapper.text()).toContain('wave4-run');
    expect(wrapper.text()).toContain('80.00%');
  });

  it('emits open-approval with proposal id after governed removal', async () => {
    (desktop.bmlRemoveMemory as ReturnType<typeof vi.fn>).mockResolvedValue({
      status: 'ok',
      proposal_id: 'prop-42',
    });
    const wrapper = factory();
    await flushPromises();
    await wrapper.find('.memory-list-item').trigger('click');
    await flushPromises();
    await wrapper.find('.memory-remove-button').trigger('click');
    await flushPromises();
    expect(appPrompt).toHaveBeenCalled();
    expect(desktop.bmlRemoveMemory).toHaveBeenCalledWith('mem-1', 'stale fact');
    expect(wrapper.emitted('open-approval')).toEqual([['prop-42']]);
    expect(wrapper.find('.memory-list-item').exists()).toBe(false);
  });

  it('does not remove when the prompt is cancelled', async () => {
    appPrompt.mockResolvedValue(null);
    const wrapper = factory();
    await flushPromises();
    await wrapper.find('.memory-list-item').trigger('click');
    await flushPromises();
    await wrapper.find('.memory-remove-button').trigger('click');
    await flushPromises();
    expect(desktop.bmlRemoveMemory).not.toHaveBeenCalled();
    expect(wrapper.find('.memory-list-item').exists()).toBe(true);
  });

  it('shows an error state when listing fails', async () => {
    (desktop.bmlListMemories as ReturnType<typeof vi.fn>).mockRejectedValue(
      new Error('backend down'),
    );
    const wrapper = factory();
    await flushPromises();
    expect(wrapper.text()).toContain('backend down');
  });
});
