import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import ThinkingBlock from './ThinkingBlock.vue';

function mountThinkingBlock() {
  return mount(ThinkingBlock, {
    props: { content: '先分析界面结构。' },
    global: {
      mocks: {
        $t: (key: string) => ({
          'chat.thinkingProcess': '思考过程',
          'chat.viewDetails': '展开',
          'chat.hideDetails': '收起',
          'chat.copy': '复制',
          'chat.copied': '已复制',
        })[key] ?? key,
      },
    },
  });
}

describe('ThinkingBlock actions', () => {
  it('shows a dedicated expand control without a copy action', async () => {
    const wrapper = mountThinkingBlock();
    const expand = wrapper.find('.thinking-expand-btn');

    expect(wrapper.find('.thinking-copy-btn').exists()).toBe(false);
    expect(expand.exists()).toBe(true);
    expect(expand.text()).toContain('展开');
    expect(expand.attributes('aria-expanded')).toBe('false');

    await expand.trigger('click');
    expect(expand.attributes('aria-expanded')).toBe('true');
    expect(wrapper.find('.thinking-content-wrapper').isVisible()).toBe(true);
  });

  it('supports controlled expansion for chat-level display preferences', async () => {
    const wrapper = mount(ThinkingBlock, {
      props: { content: '受控思考内容。', expanded: false },
      global: {
        mocks: {
          $t: (key: string) => ({
            'chat.thinkingProcess': '思考过程',
            'chat.viewDetails': '展开',
            'chat.hideDetails': '收起',
          })[key] ?? key,
        },
      },
    });

    await wrapper.find('.thinking-expand-btn').trigger('click');
    expect(wrapper.emitted('update:expanded')).toEqual([[true]]);
    expect(wrapper.find('.thinking-expand-btn').attributes('aria-expanded')).toBe('false');

    await wrapper.setProps({ expanded: true });
    expect(wrapper.find('.thinking-expand-btn').attributes('aria-expanded')).toBe('true');
  });
});
