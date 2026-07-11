import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import PlanHistoryCard from './PlanHistoryCard.vue';
import type { PlanRuntimeState } from '../../api/planning';

const plan: PlanRuntimeState = {
  plan_id: 'plan-history-1', revision: 2, title: 'Historical plan', goal: 'Restore a plan report', phase: 'Execute', status: 'InProgress', strategy: 'Inspect before execution', summary: 'Historical plan',
  steps: [{ id: 'step-1', ordinal: 0, title: 'Inspect configuration', rationale: 'Reduce risk', expected_output: 'Inspection result', status: 'Completed' }],
  todos: [{ id: 'todo-1', plan_step_id: 'step-1', title: 'Check configuration', detail: 'Inspect detailed configuration', status: 'Completed', priority: 'High', evidence_ref: 'test output', block_reason: null, updated_at: '2026-07-10T00:00:00Z' }],
  created_at: '2026-07-10T00:00:00Z', updated_at: '2026-07-10T00:00:00Z',
};

describe('PlanHistoryCard', () => {
  it('renders one markdown plan document without TODO execution details', () => {
    const wrapper = mount(PlanHistoryCard, { props: { plan } });
    expect(wrapper.text()).toContain('计划');
    expect(wrapper.text()).toContain('Historical plan');
    expect(wrapper.text()).toContain('Inspect before execution');
    expect(wrapper.text()).toContain('计划步骤');
    expect(wrapper.text()).not.toContain('Check configuration');
    expect(wrapper.text()).not.toContain('test output');
  });
});
