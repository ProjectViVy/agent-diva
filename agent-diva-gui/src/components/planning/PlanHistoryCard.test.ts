import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import PlanHistoryCard from './PlanHistoryCard.vue';
import type { PlanRuntimeState } from '../../api/planning';

vi.mock('lucide-vue-next', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    ChevronDown: icon('ChevronDown'), ChevronRight: icon('ChevronRight'),
    ClipboardList: icon('ClipboardList'), CheckCircle2: icon('CheckCircle2'),
    Clock: icon('Clock'), Loader2: icon('Loader2'), Lock: icon('Lock'),
  };
});

const plan: PlanRuntimeState = {
  plan_id: 'plan-history-1', title: '历史计划', goal: '恢复计划记录',
  phase: 'Execute', status: 'InProgress', strategy: '先检查再执行', summary: '历史计划',
  steps: [{ id: 'step-1', ordinal: 0, title: '检查', rationale: '降低风险', expected_output: '检查结果', status: 'Completed' }],
  todos: [{ id: 'todo-1', plan_step_id: 'step-1', title: '检查配置', detail: '查看详细配置', status: 'Completed', priority: 'High', evidence_ref: null, block_reason: null, updated_at: '2026-07-10T00:00:00Z' }],
  created_at: '2026-07-10T00:00:00Z', updated_at: '2026-07-10T00:00:00Z',
};

describe('PlanHistoryCard', () => {
  it('renders a persisted snapshot and expands details', async () => {
    const wrapper = mount(PlanHistoryCard, { props: { plan } });
    expect(wrapper.text()).toContain('历史计划');
    expect(wrapper.text()).toContain('检查配置');
    expect(wrapper.text()).not.toContain('查看详细');
    await wrapper.get('.plan-history-toggle').trigger('click');
    expect(wrapper.text()).toContain('查看详细');
    expect(wrapper.text()).toContain('先检查再执行');
  });
});
