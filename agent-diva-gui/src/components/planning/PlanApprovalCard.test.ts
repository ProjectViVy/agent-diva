import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import PlanApprovalCard from './PlanApprovalCard.vue';
import type { PlanRuntimeState } from '../../api/planning';

vi.mock('@lucide/vue', () => {
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
  '## 范围',
  '发布相关配置与验证流程',
  '',
  '## 计划步骤',
  '1. 检查依赖',
  '2. 运行验证',
  '',
  '## 风险与假设',
  '假设依赖源可用；失败时保留现有版本。',
  '',
  '## 验证方法',
  '运行聚焦测试和发布 smoke test。',
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
  it('shows the report shell and expands the markdown plan', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });

    expect(wrapper.text()).toContain('发布准备计划');
    expect(wrapper.text()).not.toContain('检查依赖');

    await wrapper.get('.plan-approval-details-toggle').trigger('click');
    expect(wrapper.text()).toContain('检查依赖');
    expect(wrapper.text()).toContain('运行验证');
  });

  it('emits approve and edit actions', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });

    await wrapper.get('.plan-approval-approve').trigger('click');
    await wrapper.get('.plan-approval-revoke').trigger('click');
    await wrapper.get('.plan-approval-feedback textarea').setValue('补充验证方法');
    await wrapper.get('.plan-approval-feedback .plan-approval-revoke').trigger('click');

    expect(wrapper.emitted('approve')?.[0]).toEqual([{
      contextPolicy: 'compact',
      todoPolicy: 'Optional',
      materializeTodos: false,
    }]);
    expect(wrapper.emitted('revoke')?.[0]).toEqual(['补充验证方法']);
  });

  it('passes the selected execution context policy', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });
    await wrapper.get('input[value="clear"]').setValue();
    await wrapper.get('.plan-approval-approve').trigger('click');
    expect(wrapper.emitted('approve')?.[0]).toEqual([{
      contextPolicy: 'clear',
      todoPolicy: 'Optional',
      materializeTodos: false,
    }]);
  });

  it('disables approval and requests a refresh when revision is unavailable', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan: { ...plan, revision: null } } });
    expect(wrapper.get('.plan-approval-approve').attributes('disabled')).toBeDefined();
    await wrapper.get('.plan-approval-refresh').trigger('click');
    expect(wrapper.emitted('refresh')).toHaveLength(1);
  });

  it('blocks approval for incomplete freeform plan markdown', () => {
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
    expect(wrapper.text()).toContain('计划不完整，请编辑补全后再批准');
    expect(wrapper.get('.plan-approval-approve').attributes('disabled')).toBeDefined();
  });

  it('passes explicit TODO materialization choices', async () => {
    const wrapper = mount(PlanApprovalCard, { props: { plan } });
    await wrapper.get('input[value="Always"]').setValue();
    await wrapper.get('.plan-approval-approve').trigger('click');
    expect(wrapper.emitted('approve')?.[0]).toEqual([{
      contextPolicy: 'compact',
      todoPolicy: 'Always',
      materializeTodos: true,
    }]);
  });
});
