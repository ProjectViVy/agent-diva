import { shallowMount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChatView from './ChatView.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, number>) =>
      key === 'chat.retrying'
        ? `未响应，重试 (${params?.attempt}/${params?.max})`
        : key,
  }),
}));

describe('ChatView provider status badges', () => {
  it('renders the retry badge with attempt/max on a streaming message', () => {
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [
          {
            id: 'm1',
            role: 'agent',
            content: '',
            isStreaming: true,
            retryStatus: { attempt: 2, maxRetries: 3 },
          },
        ],
        isTyping: true,
      },
    });

    expect(wrapper.text()).toContain('未响应，重试 (2/3)');
  });

  it('renders the stalled hint on a streaming message', () => {
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [
          { id: 'm2', role: 'agent', content: '', isStreaming: true, stalled: true },
        ],
        isTyping: true,
      },
    });

    expect(wrapper.text()).toContain('chat.stalled');
  });

  it('hides the badges once streaming completes', () => {
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [
          {
            id: 'm3',
            role: 'agent',
            content: 'done',
            isStreaming: false,
            retryStatus: { attempt: 1, maxRetries: 3 },
            stalled: true,
          },
        ],
        isTyping: false,
      },
    });

    expect(wrapper.text()).not.toContain('未响应');
    expect(wrapper.text()).not.toContain('chat.stalled');
  });
});
