import { mount } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ConversationSidebar from './ConversationSidebar.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      if (key === 'convSidebar.minutesAgo') return `${params?.count ?? 0}m`;
      if (key === 'convSidebar.hoursAgo') return `${params?.count ?? 0}h`;
      if (key === 'convSidebar.daysAgo') return `${params?.count ?? 0}d`;
      if (key === 'convSidebar.justNow') return 'just now';
      if (key === 'convSidebar.unknownTime') return 'unknown';
      return key;
    },
  }),
}));

vi.mock('@lucide/vue', () => {
  const stub = (name: string) => ({ name, template: `<span class="${name}" />` });
  return {
    Search: stub('Search'),
    Plus: stub('Plus'),
    Pin: stub('Pin'),
    PinOff: stub('PinOff'),
    Trash2: stub('Trash2'),
    Edit3: stub('Edit3'),
    CheckCircle2: stub('CheckCircle2'),
    XCircle: stub('XCircle'),
    Loader2: stub('Loader2'),
    MessageSquare: stub('MessageSquare'),
    X: stub('X'),
    RefreshCw: stub('RefreshCw'),
    ClipboardList: stub('ClipboardList'),
    ChevronDown: stub('ChevronDown'),
    ChevronRight: stub('ChevronRight'),
  };
});

function mountSidebar() {
  return mount(ConversationSidebar, {
    props: {
      activeSessionKey: 'gui:1',
      themeMode: 'love',
      sessions: [
        {
          session_key: 'gui:1',
          chat_id: '1',
          snippet: 'draft snippet',
          last_message: 'assistant preview',
          timestamp: Date.now(),
          title: 'LLM Title',
          message_count: 2,
          title_generated: true,
          title_manually_set: false,
          pinned: false,
        },
        {
          session_key: 'gui:2',
          chat_id: '2',
          snippet: 'second snippet',
          last_message: 'contains search target',
          timestamp: Date.now() - 1_000,
          title: 'Manual Name',
          message_count: 1,
          title_generated: false,
          title_manually_set: true,
          pinned: false,
        },
      ],
    },
  });
}

describe('ConversationSidebar', () => {
  it('renders title as primary text and last message as preview', () => {
    const wrapper = mountSidebar();
    const firstItem = wrapper.find('.conv-item');

    expect(firstItem.find('.conv-item-title').text()).toBe('LLM Title');
    expect(firstItem.find('.conv-item-preview').text()).toBe('assistant preview');
    expect(firstItem.find('.conv-item-count').text()).toBe('2');
  });

  it('filters by title and last message text', async () => {
    const wrapper = mountSidebar();
    const input = wrapper.find('.conv-search-input');

    await input.setValue('search target');
    expect(wrapper.findAll('.conv-item')).toHaveLength(1);
    expect(wrapper.find('.conv-item-title').text()).toBe('Manual Name');
  });

  it('offers a refresh action when no sessions are available', async () => {
    const wrapper = mount(ConversationSidebar, {
      props: { activeSessionKey: '', themeMode: 'love', sessions: [] },
    });

    await wrapper.find('.conv-empty-refresh').trigger('click');
    expect(wrapper.emitted('refresh')).toHaveLength(1);
  });

  it('shows an optimistic draft conversation item immediately', () => {
    const wrapper = mount(ConversationSidebar, {
      props: {
        activeSessionKey: 'gui:draft',
        themeMode: 'love',
        sessions: [
          {
            session_key: 'gui:draft',
            chat_id: 'draft',
            snippet: '新会话',
            timestamp: Date.now(),
            title: '新会话',
            last_message: undefined,
            message_count: 0,
            title_generated: false,
            title_manually_set: false,
            pinned: false,
          },
        ],
      },
    });

    expect(wrapper.find('.conv-item-title').text()).toBe('新会话');
  });
});
