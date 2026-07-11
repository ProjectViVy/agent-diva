import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import PlanApprovalCard from './PlanApprovalCard.vue';
import type { PlanRuntimeState } from '../../api/planning';

vi.mock('lucide-vue-next', () => {
  const icon = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    Check: icon('Check'),
    ChevronDown: icon('ChevronDown'),
    ChevronRight: icon('ChevronRight'),
    ClipboardList: icon('ClipboardList'),
    Loader2: icon('Loader2'),
    Pencil: icon('Pencil'),
    RefreshCw: icon('RefreshCw'),
  };
});

const plan: PlanRuntimeState = {
  plan_id: 'plan-1',
  revision: 1,
  title: '发布准备计划',
  goal: '完成发布前检查',
  phase: 'AwaitingApproval',
  status: 'Pending',
  strategy: '先检查依赖，再执行验证。',
  summary: '发布准备计划：完成发布前检查',
  steps: [{
    id: 'step-1',
    ordinal: 0,
    title: '检查依赖',
    rationale: '避免遗漏运行时依赖',
    expected_output: '依赖检查报告',
    status: 'Pending',
  }],
  todos: [],
  created_at: '2026-07-10T00:00:00Z',
  updated_at: '2026-07-10T00:00:00Z',
};

describe('PlanApprovalCard', () => {
  it('shows the plan and expands the detail preview', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });

    expect(wrapper.text()).toContain('发布准备计划');
    expect(wrapper.text()).toContain('检查依赖');
    expect(wrapper.text()).not.toContain('先检查依赖');

    await wrapper.get('.plan-approval-details-toggle').trigger('click');
    expect(wrapper.text()).toContain('先检查依赖');
    expect(wrapper.text()).toContain('依赖检查报告');
  });

  it('emits approve and revoke actions', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });

    await wrapper.get('.plan-approval-approve').trigger('click');
    await wrapper.get('.plan-approval-revoke').trigger('click');
    await wrapper.get('.plan-approval-feedback textarea').setValue('补充验证步骤');
    await wrapper.get('.plan-approval-feedback .plan-approval-revoke').trigger('click');

    expect(wrapper.emitted('approve')).toHaveLength(1);
    expect(wrapper.emitted('approve')?.[0]).toEqual([{ contextPolicy: 'compact' }]);
    expect(wrapper.emitted('revoke')).toHaveLength(1);
    expect(wrapper.emitted('revoke')?.[0]).toEqual(['补充验证步骤']);
  });

  it('lets the user select a display-only execution context policy', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });
    await wrapper.get('input[value="clear"]').setValue();
    expect(wrapper.text()).toContain('本次不会改变运行时上下文');
    await wrapper.get('.plan-approval-approve').trigger('click');
    expect(wrapper.emitted('approve')?.[0]).toEqual([{ contextPolicy: 'clear' }]);
  });

  it('disables approval and requests a refresh when revision is unavailable', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan: { ...plan, revision: null } } });
    expect(wrapper.get('.plan-approval-approve').attributes('disabled')).toBeDefined();
    await wrapper.get('.plan-approval-refresh').trigger('click');
    expect(wrapper.emitted('refresh')).toHaveLength(1);
  });
});
