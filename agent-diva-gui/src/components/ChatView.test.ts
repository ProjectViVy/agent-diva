import { flushPromises, shallowMount, type VueWrapper } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import ChatView from './ChatView.vue';

const triggerAutoDream = vi.hoisted(() => vi.fn());
vi.mock('../api/desktop', () => ({
  triggerAutoDream,
  uploadFile: vi.fn(),
}));

vi.mock('vue-i18n', () => ({
  useI18n: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      const value = ({
      'chat.toolRunning': '正在调用工具...',
      'chat.toolCall': '调用工具：{name}',
      'chat.cleanToolCall': '工具调用：{name}',
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
      'chat.deepThinking': '深度思考',
      'app.unknownTool': '未知工具',
      'general.cleanMode': '清爽模式',
      'general.cleanModeDesc': '隐藏工具输出',
      'general.cleanModeOverridesAutoExpand': '自动展开设置暂不生效',
      'chatGovernance.triggerManual': '手动触发 AutoDream',
      'chatGovernance.triggering': '正在触发 AutoDream',
      'chatGovernance.triggerNotice': 'AutoDream 已触发，请在进化页面查看实时进度。',
      'chatGovernance.openAutodream': '查看 AutoDream 实时进度',
      'chatGovernance.backendUnavailable': 'AutoDream 后端当前不可用。',
    } as Record<string, string>)[key] ?? key;
      return value.replace(/\{(\w+)\}/g, (_, name: string) => String(params?.[name] ?? `{${name}}`));
    },
  }),
}));

function mountChat(
  messages: Array<Record<string, unknown>>,
  props: Record<string, unknown> = {},
) {
  return shallowMount(ChatView, {
    props: {
      messages,
      isTyping: true,
      ...props,
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
  it('shows a short AutoDream trigger notice and links to the persisted run', async () => {
    triggerAutoDream.mockResolvedValue({ id: 'run-chat-1' });
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
      },
    });

    const triggerButton = wrapper.findAll('.toolbar-btn').find((button) =>
      button.attributes('title') === '手动触发 AutoDream');
    await triggerButton?.trigger('click');
    await flushPromises();

    expect(wrapper.find('.autodream-trigger-notice').text()).toContain('AutoDream 已触发');
    await wrapper.find('.autodream-trigger-notice__action').trigger('click');
    expect(wrapper.emitted('open-evolution')?.[0]).toEqual([
      { tab: 'autodream', sourceRunId: 'run-chat-1' },
    ]);
  });

  it('does not show a success notice when AutoDream trigger fails', async () => {
    triggerAutoDream.mockRejectedValueOnce(new Error('gateway unavailable'));
    const wrapper = shallowMount(ChatView, {
      props: {
        messages: [],
        isTyping: false,
      },
    });

    const triggerButton = wrapper.findAll('.toolbar-btn').find((button) =>
      button.attributes('title') === '手动触发 AutoDream');
    await triggerButton?.trigger('click');
    await flushPromises();

    expect(wrapper.find('.autodream-trigger-notice').text()).toContain('gateway unavailable');
    expect(wrapper.find('.autodream-trigger-notice__action').exists()).toBe(false);
  });

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

  it('renders a running tool with its name and progress dots', () => {
    const wrapper = mountChat([
      {
        id: 'tool-1',
        role: 'tool',
        content: '正在调用工具...',
        toolName: 'shell',
        toolStatus: 'running',
      },
    ]);

    expect(wrapper.text()).toContain('调用工具：shell');
    expect(wrapper.find('.streaming-dots-only').exists()).toBe(false);
    expect(wrapper.findAll('.tool-streaming-dots i')).toHaveLength(3);
  });

  it('hides completed clean-mode process messages after the process ends', () => {
    const wrapper = mountChat([
      {
        id: 'reasoning-clean',
        role: 'agent',
        content: '',
        reasoning: 'secret reasoning text',
        isStreaming: false,
      },
      {
        id: 'tool-clean-1',
        role: 'tool',
        content: 'tool completed',
        toolName: 'file_read',
        toolArgs: '{"path":"README.md"}',
        toolResult: 'secret output preview',
        toolStatus: 'success',
      },
      {
        id: 'placeholder-clean',
        role: 'agent',
        content: '',
        isStreaming: false,
      },
      {
        id: 'tool-clean-2',
        role: 'tool',
        content: 'tool failed',
        toolName: 'shell',
        toolResult: 'another secret output',
        toolStatus: 'error',
      },
      {
        id: 'answer-clean',
        role: 'agent',
        content: '最终答案',
        reasoning: 'another secret reasoning text',
      },
    ], {
      historyPrefs: {
        cleanMode: true,
        autoExpandReasoning: true,
        autoExpandToolDetails: true,
        showRawMetaByDefault: true,
      },
    });

    expect(wrapper.findAll('.clean-thinking-bubble')).toHaveLength(0);
    expect(wrapper.find('.clean-thinking-tool-list').exists()).toBe(false);
    expect(wrapper.find('.tool-message').exists()).toBe(false);
    expect(wrapper.findComponent({ name: 'ThinkingBlock' }).exists()).toBe(false);
    expect(wrapper.text()).not.toContain('secret reasoning text');
    expect(wrapper.text()).not.toContain('another secret reasoning text');
    expect(wrapper.text()).not.toContain('secret output preview');
    expect(wrapper.text()).not.toContain('another secret output');
    expect(wrapper.findComponent({ name: 'AgentMessageBody' }).props('content')).toBe('最终答案');
  });

  it('shows a single active thinking bubble for a running clean-mode tool', () => {
    const wrapper = mountChat([
      {
        id: 'tool-clean-running',
        role: 'tool',
        content: '正在调用工具...',
        toolName: 'shell',
        toolStatus: 'running',
      },
    ], {
      historyPrefs: {
        cleanMode: true,
        autoExpandReasoning: false,
        autoExpandToolDetails: false,
        showRawMetaByDefault: false,
      },
    });

    expect(wrapper.findAll('.clean-thinking-bubble')).toHaveLength(1);
    expect(wrapper.findAll('.clean-thinking-dots i')).toHaveLength(3);
    expect(wrapper.find('.clean-thinking-tool-list').text()).toBe('工具调用：shell');
    expect(wrapper.find('.clean-thinking-tool-list').text()).not.toContain('·');
    expect(wrapper.find('.tool-message').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('调用工具：shell');
    expect(wrapper.text()).toContain('工具调用：shell');
    expect(wrapper.text()).not.toContain('调用成功');
  });

  it('shows only the current clean-mode process snapshot', () => {
    const wrapper = mountChat([
      {
        id: 'tool-clean-finished',
        role: 'tool',
        content: 'tool completed',
        toolName: 'file_read',
        toolStatus: 'success',
      },
      {
        id: 'thinking-clean-current',
        role: 'agent',
        content: '',
        isStreaming: false,
      },
      {
        id: 'tool-clean-current',
        role: 'tool',
        content: '正在调用工具...',
        toolName: 'shell',
        toolStatus: 'running',
      },
    ], {
      historyPrefs: {
        cleanMode: true,
        autoExpandReasoning: true,
        autoExpandToolDetails: true,
        showRawMetaByDefault: true,
      },
    });

    expect(wrapper.findAll('.clean-thinking-bubble')).toHaveLength(1);
    expect(wrapper.find('.clean-thinking-tool-list').text()).toBe('工具调用：shell');
    expect(wrapper.find('.clean-thinking-tool-list').text()).not.toContain('file_read');
    expect(wrapper.text()).not.toContain('·');
  });

  it('removes the clean-mode thinking bubble when the answer starts', async () => {
    const wrapper = mountChat([
      {
        id: 'thinking-clean-running',
        role: 'agent',
        content: '',
        reasoning: 'secret reasoning text',
        isThinking: true,
        isStreaming: true,
      },
    ], {
      historyPrefs: {
        cleanMode: true,
        autoExpandReasoning: true,
        autoExpandToolDetails: true,
        showRawMetaByDefault: true,
      },
    });

    expect(wrapper.find('.clean-thinking-bubble').exists()).toBe(true);
    expect(wrapper.findAll('.clean-thinking-dots i')).toHaveLength(3);
    expect(wrapper.find('.clean-thinking-reasoning').text()).toBe('深度思考：secret reasoning text');

    await wrapper.setProps({
      messages: [{
        id: 'thinking-clean-running',
        role: 'agent',
        content: '',
        reasoning: '好了，我又看到了问题所在了....',
        isThinking: true,
        isStreaming: true,
      }],
    });
    await wrapper.vm.$nextTick();

    expect(wrapper.find('.clean-thinking-reasoning').text()).toBe('深度思考：好了，我又看到了问题所在了....');

    await wrapper.setProps({
      messages: [{
        id: 'answer-clean-current',
        role: 'agent',
        content: '最终答案',
        isThinking: false,
        isStreaming: false,
      }],
    });
    await wrapper.vm.$nextTick();

    expect(wrapper.find('.clean-thinking-bubble').exists()).toBe(false);
    expect(wrapper.findComponent({ name: 'AgentMessageBody' }).props('content')).toBe('最终答案');
  });

  it('applies clean mode to existing reasoning and tool expansion state', async () => {
    const wrapper = mountChat([
      {
        id: 'reasoning-existing',
        role: 'agent',
        content: 'done',
        reasoning: 'internal reasoning',
      },
      {
        id: 'tool-existing',
        role: 'tool',
        content: 'tool completed',
        toolName: 'shell',
        toolResult: 'result',
        toolStatus: 'success',
      },
    ], {
      historyPrefs: {
        cleanMode: false,
        autoExpandReasoning: true,
        autoExpandToolDetails: true,
        showRawMetaByDefault: false,
      },
    });

    await wrapper.vm.$nextTick();
    expect(wrapper.findComponent({ name: 'ThinkingBlock' }).props('expanded')).toBe(true);
    expect(wrapper.find('.tool-message .border-t').exists()).toBe(true);

    await wrapper.setProps({
      historyPrefs: {
        cleanMode: true,
        autoExpandReasoning: true,
        autoExpandToolDetails: true,
        showRawMetaByDefault: true,
      },
    });
    await wrapper.vm.$nextTick();

    expect(wrapper.findComponent({ name: 'ThinkingBlock' }).exists()).toBe(false);
    expect(wrapper.find('.tool-message .border-t').exists()).toBe(false);
    expect(wrapper.find('.clean-thinking-bubble').exists()).toBe(false);
    expect(wrapper.text()).not.toContain('internal reasoning');
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
