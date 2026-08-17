import { shallowMount, type VueWrapper } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChatView from './ChatView.vue';

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string) => ({
      'chat.toolRunning': '正在调用工具...',
      'chat.toolSuccess': '调用成功',
      'chat.toolFailed': '调用失败',
      'chat.thinking': '正在深度思考...',
      'chat.viewDetails': '查看详情',
      'chat.hideDetails': '收起详情',
      'chat.artifactSize': '结果大小:',
      'chat.artifactId': 'Artifact ID:',
      'chat.readHint': '读取提示:',
      'chat.copyArtifactReference': '复制 artifact 引用',
      'chat.compactionRunning': '正在压缩上下文…',
      'chat.compactionCompleted': '上下文压缩已完成。',
      'chat.compactionFailed': '上下文压缩失败，原上下文已保留。',
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

function mockChatScroll(
  wrapper: VueWrapper,
  initial: { scrollTop: number; scrollHeight: number; clientHeight: number },
) {
  const chatList = wrapper.get('.chat-list').element as HTMLElement;
  let scrollTop = initial.scrollTop;
  let scrollHeight = initial.scrollHeight;

  Object.defineProperties(chatList, {
    clientHeight: { configurable: true, value: initial.clientHeight },
    scrollHeight: { configurable: true, get: () => scrollHeight },
    scrollTop: {
      configurable: true,
      get: () => scrollTop,
      set: (value: number) => { scrollTop = value; },
    },
  });

  return {
    getScrollTop: () => scrollTop,
    setScrollTop: (value: number) => { scrollTop = value; },
    setScrollHeight: (value: number) => { scrollHeight = value; },
  };
}

describe('ChatView streaming states', () => {
  it('restores and persists the permission mode via localStorage', async () => {
    localStorage.setItem('agent-diva.permissionMode', 'trusted');
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
      },
    });
    expect((wrapper.vm as unknown as { permissionMode: string }).permissionMode).toBe('trusted');

    (wrapper.vm as unknown as { permissionMode: string }).permissionMode = 'cautious';
    await wrapper.vm.$nextTick();
    expect(localStorage.getItem('agent-diva.permissionMode')).toBe('cautious');
    localStorage.clear();
  });

  it('does not render approval cards inline (approval UI lives in drawer)', () => {
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
      },
    });
    expect(wrapper.findAllComponents({ name: 'ApprovalCenterCard' })).toHaveLength(0);
  });

  it('places the approval center icon under history and emits open updates', async () => {
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
        approvalCenterOpen: false,
        approvalPendingCount: 2,
      },
    });

    expect(wrapper.find('.chat-corner-actions').exists()).toBe(true);
    expect(wrapper.find('.approval-center-icon-btn').exists()).toBe(true);
    expect(wrapper.find('.approval-pending-badge').text()).toBe('2');

    await wrapper.find('.approval-center-icon-btn').trigger('click');
    expect(wrapper.emitted('update:approval-center-open')?.[0]).toEqual([true]);
  });

  it('keeps an approved plan recoverable when stream startup fails', async () => {
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
        approvingPlan: false,
        planExecutionError: 'stream startup failed',
        activePlanRuntime: {
          plan_id: 'plan-1',
          revision: 1,
          title: 'Plan 1',
          goal: 'Ship it',
          phase: 'Execute',
          status: 'Approved',
          strategy: '# Plan',
          summary: '# Plan',
          steps: [],
          todos: [],
          created_at: '2026-08-18T00:00:00Z',
          updated_at: '2026-08-18T00:00:00Z',
        },
      },
    });

    expect(wrapper.find('.active-plan-execution-error').text()).toContain('stream startup failed');
    await wrapper.get('.active-plan-execution-error button').trigger('click');
    expect(wrapper.emitted('resume-plan')).toHaveLength(1);
  });

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
    expect(wrapper.find('.chat-bubble-has-reasoning').exists()).toBe(true);
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

  it('renders ToolResultRef preview and copies a real artifact reference', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    const toolResultRef = JSON.stringify({
      version: 1,
      artifact_id: 'artifact-123',
      tool_call_id: 'call-123',
      tool_name: 'shell',
      status: 'ok',
      char_count: 2048,
      byte_count: 2100,
      sha256: 'a'.repeat(64),
      preview: 'first lines only',
      truncated: true,
      read_hint: 'agent-diva artifacts read artifact-123',
    });
    const wrapper = mountChat([{
      id: 'tool-ref',
      role: 'tool',
      content: 'tool completed',
      toolName: 'shell',
      toolStatus: 'success',
      toolResult: toolResultRef,
    }]);

    expect(wrapper.text()).toContain('first lines only');
    expect(wrapper.text()).not.toContain('"artifact_id"');
    const detailsButton = wrapper.findAll('button').find((button) => button.text().includes('查看详情'));
    expect(detailsButton).toBeDefined();
    await detailsButton!.trigger('click');
    expect(wrapper.text()).toContain('artifact-123');
    const copyButton = wrapper.findAll('button').find((button) => button.text().includes('复制 artifact 引用'));
    expect(copyButton).toBeDefined();
    await copyButton!.trigger('click');
    expect(writeText).toHaveBeenCalledWith('artifact://artifact-123');
  });

  it('shows one non-spinning failed compaction status with context-retention text', () => {
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
        compactionStatus: {
          trigger: 'reactive',
          phase: 'failed',
          summary: 'provider rejected the compacted context; original context retained',
        },
      },
    });
    expect(wrapper.findAll('.compaction-status-line')).toHaveLength(1);
    expect(wrapper.text()).toContain('原上下文已保留');
    expect(wrapper.text()).toContain('original context retained');
    expect(wrapper.find('.compaction-status-line .animate-spin').exists()).toBe(false);
  });

  it('opens session history as an overlay on narrow windows', async () => {
    const originalWidth = window.innerWidth;
    Object.defineProperty(window, 'innerWidth', { configurable: true, value: 900 });
    const wrapper = mountChat([]);

    await wrapper.find('.chat-corner-actions .toolbar-btn').trigger('click');

    expect(wrapper.find('.conv-sidebar-scrim').exists()).toBe(true);
    expect(wrapper.find('.conv-sidebar-wrapper--overlay').exists()).toBe(true);

    Object.defineProperty(window, 'innerWidth', { configurable: true, value: originalWidth });
  });

  it('preserves the reading position during streaming and ask-user updates', async () => {
    const wrapper = mountChat([{
      id: 'streaming',
      role: 'agent',
      content: 'Initial response',
      isStreaming: true,
    }]);
    await wrapper.vm.$nextTick();
    await wrapper.vm.$nextTick();
    const scroll = mockChatScroll(wrapper, {
      scrollTop: 200,
      scrollHeight: 1000,
      clientHeight: 200,
    });
    await wrapper.get('.chat-list').trigger('scroll');

    scroll.setScrollHeight(1200);
    await wrapper.setProps({
      messages: [{
        id: 'streaming',
        role: 'agent',
        content: 'Initial response with a much longer streamed continuation',
        isStreaming: true,
      }],
    });
    await wrapper.vm.$nextTick();
    expect(scroll.getScrollTop()).toBe(200);

    scroll.setScrollHeight(1350);
    await wrapper.setProps({
      askUserQuestions: [{
        question_id: 'question-1',
        question: 'Choose one',
        choices: ['A', 'B'],
        allow_other: false,
        created_at: '2026-08-17T00:00:00Z',
        timeout_seconds: 60,
      }],
    });
    await wrapper.vm.$nextTick();
    expect(scroll.getScrollTop()).toBe(200);
  });

  it('resumes following updates after the user scrolls near the bottom', async () => {
    const wrapper = mountChat([{
      id: 'streaming',
      role: 'agent',
      content: 'Initial response',
      isStreaming: true,
    }]);
    await wrapper.vm.$nextTick();
    await wrapper.vm.$nextTick();
    const scroll = mockChatScroll(wrapper, {
      scrollTop: 770,
      scrollHeight: 1000,
      clientHeight: 200,
    });
    await wrapper.get('.chat-list').trigger('scroll');

    scroll.setScrollHeight(1200);
    await wrapper.setProps({
      messages: [{
        id: 'streaming',
        role: 'agent',
        content: 'Initial response with a streamed continuation',
        isStreaming: true,
      }],
    });
    await wrapper.vm.$nextTick();

    expect(scroll.getScrollTop()).toBe(1200);
  });

  it('returns to the latest message when sending or switching sessions', async () => {
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
        activeSessionKey: 'gui:first',
      },
    });
    await wrapper.vm.$nextTick();
    await wrapper.vm.$nextTick();
    const scroll = mockChatScroll(wrapper, {
      scrollTop: 100,
      scrollHeight: 1000,
      clientHeight: 200,
    });
    await wrapper.get('.chat-list').trigger('scroll');

    await wrapper.get('.chat-textarea').setValue('New message');
    await wrapper.get('.input-action-btn.send').trigger('click');
    await wrapper.vm.$nextTick();
    expect(scroll.getScrollTop()).toBe(1000);

    scroll.setScrollTop(150);
    scroll.setScrollHeight(1400);
    await wrapper.get('.chat-list').trigger('scroll');
    await wrapper.setProps({ activeSessionKey: 'gui:second' });
    await wrapper.vm.$nextTick();
    expect(scroll.getScrollTop()).toBe(1400);
  });
});
