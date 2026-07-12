import { shallowMount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChatView from './ChatView.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => ({
      'chat.toolRunning': '正在调用工具...',
      'chat.toolSuccess': '调用成功',
      'chat.toolFailed': '调用失败',
      'chat.thinking': '正在深度思考...',
    })[key] ?? key,
  }),
}));

function mountChat(messages: Array<Record<string, unknown>>) {
  return shallowMount(ChatView, {
    props: {
      messages,
      isTyping: true,
    },
  });
}

describe('ChatView streaming states', () => {
  it('keeps the thinking status separate from its loading dots', () => {
    const wrapper = mountChat([
      {
        id: 'thinking',
        role: 'agent',
        content: '',
        reasoning: '先分析问题。',
        isStreaming: true,
      },
    ]);

    expect(wrapper.find('.streaming-reasoning-section').exists()).toBe(true);
    expect(wrapper.find('.streaming-reasoning-status').text()).toContain('正在深度思考...');
    expect(wrapper.findAll('.streaming-reasoning-status .streaming-dots i')).toHaveLength(3);
    expect(wrapper.find('.streaming-dots-only').exists()).toBe(false);
  });

  it('renders a running tool as a tool card without an assistant loading bubble', () => {
    const wrapper = mountChat([
      {
        id: 'tool-1',
        role: 'tool',
        content: '正在调用工具...',
        toolName: 'shell',
        toolStatus: 'running',
      },
    ]);

    expect(wrapper.text()).toContain('正在调用工具...');
    expect(wrapper.text()).toContain('shell');
    expect(wrapper.find('.streaming-dots-only').exists()).toBe(false);
    expect(wrapper.findAll('.streaming-dots i')).toHaveLength(0);
  });
});
