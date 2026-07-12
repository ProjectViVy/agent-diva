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
          'common.copy': '复制',
          'common.copied': '已复制',
        })[key] ?? key,
      },
    },
  });
}

describe('ThinkingBlock actions', () => {
  it('keeps copy and expand as independent controls', async () => {
    const wrapper = mountThinkingBlock();
    const copy = wrapper.find('.thinking-copy-btn');
    const expand = wrapper.find('.thinking-expand-btn');

    expect(copy.exists()).toBe(true);
    expect(expand.exists()).toBe(true);
    expect(expand.text()).toContain('展开');
    expect(expand.attributes('aria-expanded')).toBe('false');

    await expand.trigger('click');
    expect(expand.attributes('aria-expanded')).toBe('true');
    expect(wrapper.find('.thinking-content-wrapper').isVisible()).toBe(true);
  });
});
