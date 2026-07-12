import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import ThinkingBlock from './ThinkingBlock.vue';

function mountThinkingBlock() {
  return mount(ThinkingBlock, {
    props: { content: 'A short thought.' },
    global: {
      mocks: {
        $t: (key: string) => key,
      },
    },
  });
}

describe('ThinkingBlock', () => {
  it('keeps copy and expand controls in a dedicated action group', () => {
    const wrapper = mountThinkingBlock();

    expect(wrapper.find('.thinking-header-actions').exists()).toBe(true);
    expect(wrapper.find('.thinking-copy-btn').attributes('aria-label')).toBe('common.copy');
    expect(wrapper.find('.thinking-chevron').exists()).toBe(true);
  });

  it('expands when the header is clicked without copying the thought', async () => {
    const wrapper = mountThinkingBlock();

    await wrapper.find('.thinking-header').trigger('click');

    expect(wrapper.find('.thinking-header-expanded').exists()).toBe(true);
  });
});
