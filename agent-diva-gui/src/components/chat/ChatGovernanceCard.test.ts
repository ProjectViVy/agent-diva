import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChatGovernanceCard from './ChatGovernanceCard.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      params ? `${key}:${JSON.stringify(params)}` : key,
  }),
}));

vi.mock('lucide-vue-next', () => ({
  AlertTriangle: { name: 'AlertTriangle', template: '<span />' },
  ArrowRight: { name: 'ArrowRight', template: '<span />' },
  CheckCircle2: { name: 'CheckCircle2', template: '<span />' },
  Clock3: { name: 'Clock3', template: '<span />' },
  FileSearch: { name: 'FileSearch', template: '<span />' },
  GitBranch: { name: 'GitBranch', template: '<span />' },
  Loader2: { name: 'Loader2', template: '<span />' },
  ShieldAlert: { name: 'ShieldAlert', template: '<span />' },
}));

describe('ChatGovernanceCard', () => {
  it('emits an inbox deep link for proposal cards', async () => {
    const wrapper = mount(ChatGovernanceCard, {
      props: {
        card: {
          kind: 'evolution_proposal',
          id: 'proposal-1',
          proposal_type: 'memory_patch',
          state: 'pending_review',
          risk_level: 'medium',
          target_section: 'memory_md',
          summary: 'Review this memory update',
          evidence_count: 2,
          source_run_id: 'run-1',
        },
      },
    });

    await wrapper.find('button').trigger('click');

    expect(wrapper.emitted('open-evolution')?.[0]).toEqual([
      { tab: 'inbox', proposalId: 'proposal-1', sourceRunId: 'run-1' },
    ]);
  });

  it('shows recoverable error and links failed run cards to Runs', async () => {
    const wrapper = mount(ChatGovernanceCard, {
      props: {
        card: {
          kind: 'autodream_run',
          id: 'run-2',
          state: 'unavailable',
          trigger: 'manual',
          proposal_ids: [],
          error: 'backend unavailable',
        },
      },
    });

    expect(wrapper.text()).toContain('backend unavailable');

    await wrapper.find('button').trigger('click');
    expect(wrapper.emitted('open-evolution')?.[0]).toEqual([
      { tab: 'runs', sourceRunId: 'run-2' },
    ]);
  });
});
