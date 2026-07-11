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
    Loader2: icon('Loader2'),
    Pencil: icon('Pencil'),
    RefreshCw: icon('RefreshCw'),
  };
});

const markdown = [
  '# 发布准备计划',
  '',
  '## 目标',
  '完成发布前检查',
  '',
  '## 计划步骤',
  '1. 检查依赖',
  '2. 运行验证',
].join('\n');

const plan: PlanRuntimeState = {
  plan_id: 'plan-1',
  revision: 1,
  title: '发布准备计划',
  goal: '完成发布前检查',
  phase: 'AwaitingApproval',
  status: 'AwaitingApproval',
  strategy: markdown,
  summary: markdown,
  markdown,
  steps: [],
  todos: [],
  created_at: '2026-07-10T00:00:00Z',
  updated_at: '2026-07-10T00:00:00Z',
};

describe('PlanApprovalCard', () => {
  it('shows the full plan body by default (not goal-only)', () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });

    expect(wrapper.text()).toContain('待审批');
    expect(wrapper.text()).toContain('发布准备计划');
    expect(wrapper.text()).toContain('完成发布前检查');
    expect(wrapper.text()).toContain('检查依赖');
    expect(wrapper.text()).toContain('运行验证');
    // Context options stay collapsed until expanded.
    expect(wrapper.text()).not.toContain('压缩上下文');
  });

  it('resolves a generic Plan title from markdown H1', () => {
    const wrapper = mount(PlanApprovalCard, {
      props: {
        plan: {
          ...plan,
          title: 'Plan',
          goal: '完成发布前检查',
        },
      },
    });
    expect(wrapper.get('.plan-approval-title').text()).toBe('发布准备计划');
    expect(wrapper.text()).not.toMatch(/计划\s*Plan\s*Plan/);
  });

  it('emits approve and edit actions', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });

    await wrapper.get('.plan-approval-approve').trigger('click');
    await wrapper.get('.plan-approval-revoke').trigger('click');
    await wrapper.get('.plan-approval-feedback textarea').setValue('补充验证方法');
    await wrapper.get('.plan-approval-feedback .plan-approval-revoke').trigger('click');

    expect(wrapper.emitted('approve')?.[0]).toEqual([{ contextPolicy: 'compact' }]);
    expect(wrapper.emitted('revoke')?.[0]).toEqual(['补充验证方法']);
  });

  it('passes the selected execution context policy', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });
    await wrapper.get('.plan-approval-details-toggle').trigger('click');
    await wrapper.get('input[value="clear"]').setValue();
    await wrapper.get('.plan-approval-approve').trigger('click');
    expect(wrapper.emitted('approve')?.[0]).toEqual([{ contextPolicy: 'clear' }]);
  });

  it('disables approval and requests a refresh when revision is unavailable', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan: { ...plan, revision: null } } });
    expect(wrapper.get('.plan-approval-approve').attributes('disabled')).toBeDefined();
    await wrapper.get('.plan-approval-refresh').trigger('click');
    expect(wrapper.emitted('refresh')).toHaveLength(1);
  });

  it('still shows approve for incomplete freeform plan markdown', () => {
    const incompleteMarkdown = [
      '# C++ 测试项目',
      '',
      '## 目标',
      '创建项目',
      '',
      '## 范围',
      'cpp/',
    ].join('\n');
    const incomplete: PlanRuntimeState = {
      ...plan,
      title: 'C++ 测试项目',
      goal: '创建项目',
      strategy: incompleteMarkdown,
      summary: incompleteMarkdown,
      markdown: incompleteMarkdown,
      validation_issues: ['计划报告缺少章节：计划步骤', '计划报告缺少章节：风险与假设', '计划报告缺少章节：验证方法'],
    };
    const wrapper = mount(PlanApprovalCard, { props: { plan: incomplete } });
    expect(wrapper.text()).toContain('章节不完整，仍可批准或点编辑继续完善');
    expect(wrapper.get('.plan-approval-approve').attributes('disabled')).toBeUndefined();
  });
});
