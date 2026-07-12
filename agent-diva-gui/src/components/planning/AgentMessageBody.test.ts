import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import AgentMessageBody from './AgentMessageBody.vue';

describe('AgentMessageBody', () => {
  it('renders proposed_plan as a plan message block without raw tags', () => {
    const content = [
      '好的，基于当前情况，以下是完整的扩展计划：',
      '',
      '<proposed_plan>',
      'hello.py 功能扩展计划',
      '目标',
      '扩展 hello.py',
      '范围',
      '只改一个文件',
      '</proposed_plan>',
    ].join('\n');

    const wrapper = mount(AgentMessageBody, { props: { content } });
    expect(wrapper.text()).toContain('计划');
    expect(wrapper.text()).toContain('Plan');
    expect(wrapper.text()).toContain('hello.py 功能扩展计划');
    expect(wrapper.text()).toContain('目标');
    expect(wrapper.text()).toContain('以下是完整的扩展计划');
    expect(wrapper.html()).not.toContain('&lt;proposed_plan&gt;');
    expect(wrapper.text()).not.toContain('<proposed_plan>');
    expect(wrapper.text()).not.toContain('</proposed_plan>');
  });

  it('renders ordinary messages as markdown without a plan card', () => {
    const wrapper = mount(AgentMessageBody, {
      props: { content: '普通助手回复。' },
    });
    expect(wrapper.text()).toContain('普通助手回复');
    expect(wrapper.find('.proposed-plan-block').exists()).toBe(false);
  });
});
