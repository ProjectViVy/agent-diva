import { mount, flushPromises } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import EvolutionView from './EvolutionView.vue';
import { listLaputaProposals, pollLaputaEvents } from '../api/desktop';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params ? `${key}:${JSON.stringify(params)}` : key,
  }),
}));

vi.mock('lucide-vue-next', () => ({
  AlertTriangle: { name: 'AlertTriangle', template: '<span class="AlertTriangle" />' },
  Archive: { name: 'Archive', template: '<span class="Archive" />' },
  ClipboardList: { name: 'ClipboardList', template: '<span class="ClipboardList" />' },
  FileClock: { name: 'FileClock', template: '<span class="FileClock" />' },
  GitBranch: { name: 'GitBranch', template: '<span class="GitBranch" />' },
  History: { name: 'History', template: '<span class="History" />' },
  RefreshCw: { name: 'RefreshCw', template: '<span class="RefreshCw" />' },
  ShieldCheck: { name: 'ShieldCheck', template: '<span class="ShieldCheck" />' },
}));

vi.mock('../api/desktop', () => ({
  listLaputaProposals: vi.fn(),
  pollLaputaEvents: vi.fn(),
}));

const proposal = {
  id: 'proposal-1',
  created_at: '2026-06-14T00:00:00Z',
  updated_at: '2026-06-14T00:00:00Z',
  created_by: 'autodream',
  proposal_type: 'memory_patch',
  target_section: 'memory_md',
  evidence_refs: [],
  proposed_patch: 'append memory',
  risk_level: 'medium',
  state: 'pending_review',
  source_run_id: null,
};

describe('EvolutionView shell', () => {
  beforeEach(() => {
    vi.mocked(listLaputaProposals).mockResolvedValue([proposal]);
    vi.mocked(pollLaputaEvents).mockResolvedValue([]);
  });

  it('defaults to Inbox and emits the pending proposal count', async () => {
    const wrapper = mount(EvolutionView);
    await flushPromises();

    expect(wrapper.find('[data-testid="evolution-tab-inbox"]').classes()).toContain('active');
    expect(wrapper.emitted('count-change')?.at(-1)?.[0]).toMatchObject({
      total: 1,
      tone: 'warning',
    });
  });

  it('renders all shell tabs with Runs, Audit, and Policy placeholders', async () => {
    const wrapper = mount(EvolutionView);
    await flushPromises();

    expect(wrapper.find('[data-testid="evolution-tab-inbox"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="evolution-tab-runs"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="evolution-tab-audit"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="evolution-tab-policy"]').exists()).toBe(true);
  });

  it('keeps explicit responsive list/detail guardrail classes on the Inbox shell', async () => {
    const wrapper = mount(EvolutionView);
    await flushPromises();

    const inbox = wrapper.find('[data-testid="evolution-inbox-shell"]');
    expect(inbox.classes()).toContain('evolution-inbox-shell');
    expect(inbox.classes()).toContain('evolution-inbox-responsive');
  });
});
