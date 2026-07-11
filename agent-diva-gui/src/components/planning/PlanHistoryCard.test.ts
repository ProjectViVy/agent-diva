import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import PlanHistoryCard from './PlanHistoryCard.vue';
import type { PlanRuntimeState } from '../../api/planning';

const markdown = [
  '# Historical plan',
  '',
  '## 目标',
  'Restore a plan report',
  '',
  '## 计划步骤',
  '1. Inspect before execution',
].join('\n');

const plan: PlanRuntimeState = {
  plan_id: 'plan-history-1',
  revision: 2,
  title: 'Historical plan',
  goal: 'Restore a plan report',
  phase: 'Execute',
  status: 'Approved',
  strategy: markdown,
  summary: markdown,
  markdown,
  steps: [{ id: 'step-1', ordinal: 0, title: 'Inspect configuration', rationale: 'Reduce risk', expected_output: 'Inspection result', status: 'Completed' }],
  todos: [{ id: 'todo-1', plan_step_id: 'step-1', title: 'Check configuration', detail: 'Inspect detailed configuration', status: 'Completed', priority: 'High', evidence_ref: 'test output', block_reason: null, updated_at: '2026-07-10T00:00:00Z' }],
  created_at: '2026-07-10T00:00:00Z',
  updated_at: '2026-07-10T00:00:00Z',
};

describe('PlanHistoryCard', () => {
  it('renders a real title once and the full plan document', () => {
    const wrapper = mount(PlanHistoryCard, { props: { plan } });
    expect(wrapper.text()).toContain('历史计划');
    expect(wrapper.get('.plan-history-card__title').text()).toBe('Historical plan');
    // Badge is 历史计划, not "计划 / Plan / Plan".
    expect(wrapper.text()).not.toMatch(/^计划\s*Plan/m);
    expect(wrapper.text()).toContain('Restore a plan report');
    expect(wrapper.text()).toContain('计划步骤');
    expect(wrapper.text()).not.toContain('Check configuration');
    expect(wrapper.text()).not.toContain('test output');
  });

  it('rebuilds incomplete title:goal snapshots into a readable document', () => {
    const stub: PlanRuntimeState = {
      ...plan,
      markdown: undefined,
      summary: 'Historical plan: Restore a plan report',
      strategy: null,
      steps: [{ id: 'step-1', ordinal: 0, title: 'Inspect configuration', rationale: null, expected_output: null, status: 'Completed' }],
      todos: [],
    };
    const wrapper = mount(PlanHistoryCard, { props: { plan: stub } });
    expect(wrapper.get('.plan-history-card__title').text()).toBe('Historical plan');
    expect(wrapper.text()).toContain('Restore a plan report');
    expect(wrapper.text()).toContain('Inspect configuration');
  });

  it('does not surface generic Plan as the card title when markdown has a real H1', () => {
    const wrapper = mount(PlanHistoryCard, {
      props: {
        plan: {
          ...plan,
          title: 'Plan',
        },
      },
    });
    expect(wrapper.get('.plan-history-card__title').text()).toBe('Historical plan');
  });
});
