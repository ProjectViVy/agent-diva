<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted, onBeforeUnmount } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Send, Square, Plus, Wrench, ChevronDown, ChevronRight, CheckCircle, CheckCircle2, XCircle, Loader2, Brain, Copy, Edit, RefreshCw, Rewind, GitFork, Paperclip, Mic, Settings2, Zap, Clock, Shield, Sparkles, Cat, GitBranch, ClipboardList } from 'lucide-vue-next';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css'; // 使用 GitHub Dark 风格
import { useI18n } from 'vue-i18n';
import ConversationSidebar from './ConversationSidebar.vue';
import DecisionCard from './DecisionCard.vue';
import TodoCard from './TodoCard.vue';
import ApprovalBanner from './ApprovalBanner.vue';
import ChatGovernanceCard from './chat/ChatGovernanceCard.vue';
import ThinkingBlock from './chat/ThinkingBlock.vue';
import ThinkingToggle from './chat/ThinkingToggle.vue';
import PlanApprovalCard from './planning/PlanApprovalCard.vue';
import AgentMessageBody from './planning/AgentMessageBody.vue';
import { activePlanTodos as filterActivePlanTodos } from './planning/planExecutionState';
import {
  triggerAutoDream,
  getAutoDreamRunStatus,
  getLaputaProposal,
  uploadFile,
  FileAttachmentDto,
  type AutoDreamRunRecord,
  type EvolutionProposal,
  type UiCard,
  type ApprovalRequest,
} from '../api/desktop';
import type { PlanRuntimeState } from '../api/planning';
import type {
  ChatGovernanceCard as ChatGovernanceCardModel,
  ChatGovernanceDeepLink,
} from './chat/governanceCards';
import type { ToolsConfigShape } from '../types/toolsConfig';
import { budgetPressurePercent, computeBudgetStatus } from '../utils/contextBudget';

const { t } = useI18n();

const escapeHtml = (text: string): string =>
  text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');

const md = new MarkdownIt({
  html: false, // 禁用 HTML 标签以防止 XSS
  linkify: true,
  breaks: true,
  highlight: function (str: string, lang: string): string {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return '<pre class="hljs"><code>' +
               hljs.highlight(str, { language: lang, ignoreIllegals: true }).value +
               '</code></pre>';
      } catch (__) {}
    }

    return '<pre class="hljs"><code>' + escapeHtml(str) + '</code></pre>';
  }
});

interface Message {
  id: string;
  role: 'user' | 'agent' | 'system' | 'tool';
  content: string;
  reasoning?: string;
  isThinking?: boolean;
  isStreaming?: boolean;
  timestamp?: number;
  emotion?: string;
  toolName?: string;
  toolArgs?: string;
  toolResult?: string;
  toolStatus?: 'running' | 'success' | 'error';
  toolCallId?: string;
  rawMeta?: Record<string, unknown>;
  fromHistory?: boolean;
  attachments?: string[];
}

const expandedTools = ref<Record<string, boolean>>({});
const expandedReasoning = ref<Record<string, boolean>>({});
const expandedRawMeta = ref<Record<string, boolean>>({});

interface HistoryPrefs {
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
}

const defaultHistoryPrefs: HistoryPrefs = {
  autoExpandReasoning: true,
  autoExpandToolDetails: false,
  showRawMetaByDefault: false,
};

const toggleTool = (messageId: string) => {
  expandedTools.value[messageId] = !expandedTools.value[messageId];
};

const toggleRawMeta = (messageId: string) => {
  expandedRawMeta.value[messageId] = !expandedRawMeta.value[messageId];
};

const hasRawMeta = (msg: Message) => {
  return !!msg.rawMeta && Object.keys(msg.rawMeta).length > 0;
};

const renderRawMeta = (msg: Message) => {
  if (!msg.rawMeta) return '';
  try {
    return JSON.stringify(msg.rawMeta, null, 2);
  } catch {
    return String(msg.rawMeta);
  }
};

interface Session {
  session_key: string;
  chat_id: string;
  snippet: string;
  timestamp: number;
  title?: string;
  last_message?: string;
  message_count: number;
  title_generated: boolean;
  title_manually_set: boolean;
  pinned?: boolean;
  status?: 'idle' | 'running' | 'completed' | 'error';
  agent_icon?: string;
  agent_name?: string;
}

const props = defineProps<{
  messages: Message[];
  isTyping: boolean;
  themeMode?: string;
  historyPrefs?: HistoryPrefs;
  sessions?: Session[];
  toolsConfig?: ToolsConfigShape;
  activeSessionKey?: string;
  activePlanRuntime?: PlanRuntimeState | null;
  pendingApprovalPlan?: PlanRuntimeState | null;
  executingPlan?: PlanRuntimeState | null;
  approvingPlan?: boolean;
}>();

const emit = defineEmits<{
  (e: 'send', content: string, attachments?: FileAttachmentDto[], mode?: 'agent' | 'plan' | 'ask'): void;
  (e: 'approve-plan', payload: { contextPolicy: 'retain' | 'compact' | 'clear' }): void;
  (e: 'revoke-plan', feedback: string): void;
  (e: 'refresh-plan'): void;
  (e: 'refresh-sessions'): void;
  (e: 'clear'): void;
  (e: 'stop'): void;
  (e: 'select-session', sessionKey: string): void;
  (e: 'delete-session', sessionKey: string): void;
  (e: 'new-session'): void;
  (e: 'toggle-pin', sessionKey: string): void;
  (e: 'rename-session', sessionKey: string, title: string): void;
  (e: 'open-evolution', payload: ChatGovernanceDeepLink): void;
  (e: 'regenerate', messageId: string): void;
}>();

const input = ref('');
const messagesEndRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLTextAreaElement | null>(null);
const fileInputRef = ref<HTMLInputElement | null>(null);
const attachments = ref<FileAttachmentDto[]>([]);
const uploading = ref(false);
const uploadingPastes = ref(false);
const inputHeight = ref(24); // 动态输入框高度

// 右侧会话侧边栏状态
const convSidebarOpen = ref(false);
const convSidebarRef = ref<InstanceType<typeof ConversationSidebar> | null>(null);
const narrowLayout = ref(false);

const updateNarrowLayout = () => {
  narrowLayout.value = window.innerWidth < 1024;
};

// 输入区域状态
const showModeMenu = ref(false);
const showPermissionMenu = ref(false);
const execMode = ref<'agent' | 'plan' | 'ask'>('agent');
const permissionMode = ref<'cautious' | 'smart' | 'trusted'>('smart');
// const showAttachments = ref(false); // 预留
const isRecording = ref(false);
// const recordingDuration = ref(0); // 预留
const thinkingMode = ref<'auto' | 'on' | 'off'>('auto');
const localGovernanceCards = ref<ChatGovernanceCardModel[]>([]);
const autoDreamTriggering = ref(false);
const autoDreamPollTimers = new Set<ReturnType<typeof setTimeout>>();

const effectiveHistoryPrefs = computed<HistoryPrefs>(() => ({
  ...defaultHistoryPrefs,
  ...(props.historyPrefs ?? {}),
}));

const contextBudgetStatus = computed(() =>
  computeBudgetStatus(props.messages, props.toolsConfig?.budget)
);

const contextUsagePercent = computed(() =>
  Math.min(100, Math.max(0, budgetPressurePercent(contextBudgetStatus.value)))
);

const contextRingDashoffset = computed(() => {
  const circumference = 56.5;
  return circumference * (1 - contextUsagePercent.value / 100);
});

const contextRingColor = computed(() => {
  if (contextUsagePercent.value >= 80) return '#f97316';
  if (contextUsagePercent.value >= 60) return '#eab308';
  return 'var(--brand, #ec4899)';
});

const contextUsageTitle = computed(() =>
  `${contextBudgetStatus.value.history_estimated.toLocaleString()} / ${contextBudgetStatus.value.history_budget.toLocaleString()}`
);

const sakura = [
  { left: '6%', top: '18%', size: 16, opacity: 0.25, delay: 0 },
  { left: '18%', top: '64%', size: 12, opacity: 0.18, delay: 0.8 },
  { left: '36%', top: '30%', size: 20, opacity: 0.22, delay: 1.2 },
  { left: '52%', top: '52%', size: 14, opacity: 0.2, delay: 0.4 },
  { left: '70%', top: '22%', size: 18, opacity: 0.24, delay: 1.6 },
  { left: '82%', top: '70%', size: 12, opacity: 0.18, delay: 0.6 },
  { left: '90%', top: '38%', size: 16, opacity: 0.2, delay: 1.1 },
];

const scrollToBottom = () => {
  nextTick(() => {
    messagesEndRef.value?.scrollIntoView({ behavior: 'smooth' });
  });
};

watch(() => props.messages, (newMessages, oldMessages) => {
  // Clear expansion states if messages are cleared (length reduced significantly or reset)
  if (oldMessages && newMessages.length < oldMessages.length) {
    expandedReasoning.value = {};
    expandedTools.value = {};
    expandedRawMeta.value = {};
    cardCache.clear();
  }

  // Auto-expand structured sections based on local preferences
  newMessages.forEach((msg) => {
    if (expandedReasoning.value[msg.id] === undefined && msg.reasoning) {
      expandedReasoning.value[msg.id] = effectiveHistoryPrefs.value.autoExpandReasoning || !!msg.isThinking;
    }
    if (expandedTools.value[msg.id] === undefined && msg.role === 'tool') {
      expandedTools.value[msg.id] = effectiveHistoryPrefs.value.autoExpandToolDetails;
    }
    if (expandedRawMeta.value[msg.id] === undefined && hasRawMeta(msg)) {
      expandedRawMeta.value[msg.id] = effectiveHistoryPrefs.value.showRawMetaByDefault;
    }
  });
  scrollToBottom();
}, { deep: true });

onMounted(() => {
  updateNarrowLayout();
  window.addEventListener('resize', updateNarrowLayout);
  scrollToBottom();
  inputRef.value?.focus();
});

onBeforeUnmount(() => {
  window.removeEventListener('resize', updateNarrowLayout);
  autoDreamPollTimers.forEach((timer) => clearTimeout(timer));
  autoDreamPollTimers.clear();
});

const handleSend = () => {
  if (props.isTyping) return;
  if (!input.value.trim() && attachments.value.length === 0) return;
  const currentAttachments = [...attachments.value];
  const text = input.value.trim() || (currentAttachments.length > 0 ? t('chat.filePlaceholder') : '');
  emit('send', text, currentAttachments.length > 0 ? currentAttachments : undefined, execMode.value);
  input.value = '';
  attachments.value = [];
  nextTick(() => {
    adjustInputHeight();
  });
};

/** Leave plan mode as soon as the user approves execution. */
function handleApprovePlan(payload: { contextPolicy: 'retain' | 'compact' | 'clear' }) {
  execMode.value = 'agent';
  showModeMenu.value = false;
  emit('approve-plan', payload);
}

// Parent may flip runtime into Execute without going through the local approve
// handler (e.g. restore / event). Keep the mode selector aligned.
watch(
  () => props.executingPlan?.plan_id ?? null,
  (executionId, previousId) => {
    if (executionId && executionId !== previousId) {
      execMode.value = 'agent';
      showModeMenu.value = false;
    }
  },
);

const handleFileSelect = async (event: Event) => {
  const target = event.target as HTMLInputElement;
  const files = target.files;
  if (!files || files.length === 0) return;
  uploading.value = true;
  try {
    for (const file of Array.from(files)) {
      const buffer = await file.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buffer));
      const dto = await uploadFile(file.name, bytes, 'gui');
      attachments.value.push(dto);
    }
  } catch (err) {
    console.error('Failed to upload file:', err);
  } finally {
    uploading.value = false;
    if (fileInputRef.value) fileInputRef.value.value = '';
  }
};

const handlePaste = async (event: ClipboardEvent) => {
  if (!event.clipboardData) return;
  const items = Array.from(event.clipboardData.items);
  const imageItems = items.filter((item) => item.type.startsWith('image/'));
  if (imageItems.length === 0) return;
  event.preventDefault();
  uploadingPastes.value = true;
  try {
    for (const item of imageItems) {
      const blob = item.getAsFile();
      if (!blob) continue;
      const buffer = await blob.arrayBuffer();
      const bytes = Array.from(new Uint8Array(buffer));
      const fileName = blob.name || 'pasted-image.png';
      const dto = await uploadFile(fileName, bytes, 'gui');
      attachments.value.push(dto);
    }
  } catch (err) {
    console.error('Failed to upload pasted image:', err);
    alert(t('chat.pasteUploadFailed') || 'Failed to upload pasted image');
  } finally {
    uploadingPastes.value = false;
  }
};

const removeAttachment = (index: number) => {
  attachments.value.splice(index, 1);
};

const handleClear = async () => {
  try {
    await invoke('reset_session', { channel: 'gui', chatId: 'main' });
  } catch (error) {
    console.error('Failed to reset session on backend:', error);
  } finally {
    emit('clear');
  }
};

const handleStop = () => {
  if (!props.isTyping) return;
  emit('stop');
};

const toRunCard = (run: AutoDreamRunRecord): ChatGovernanceCardModel => ({
  kind: 'autodream_run',
  id: run.id,
  state: run.state,
  trigger: run.trigger,
  summary: run.summary,
  proposal_ids: Array.isArray(run.proposal_ids) ? run.proposal_ids : [],
  error: run.error,
  source_run_id: run.id,
  created_at: run.started_at,
  updated_at: run.completed_at ?? run.started_at,
});

const toProposalCard = (proposal: EvolutionProposal): ChatGovernanceCardModel => ({
  kind: 'evolution_proposal',
  id: proposal.id,
  proposal_type: proposal.proposal_type,
  state: proposal.state,
  risk_level: proposal.risk_level,
  target_section: proposal.target_section,
  summary: proposal.proposed_patch.split('\n').find((line) => line.trim().length > 0) ?? proposal.proposed_patch,
  evidence_count: Array.isArray(proposal.evidence_refs) ? proposal.evidence_refs.length : 0,
  source_run_id: proposal.source_run_id,
});

const normalizeError = (error: unknown) => {
  if (error && typeof error === 'object' && 'message' in error) {
    return String((error as { message: unknown }).message);
  }
  if (error instanceof Error) return error.message;
  if (error === null || error === undefined) return t('chatGovernance.backendUnavailable');
  return String(error);
};

const replaceGovernanceCard = (id: string, next: ChatGovernanceCardModel) => {
  localGovernanceCards.value = localGovernanceCards.value.map((card) => (card.id === id ? next : card));
};

const appendProposalCards = async (run: AutoDreamRunRecord) => {
  const proposalIds = Array.isArray(run.proposal_ids) ? run.proposal_ids : [];
  if (proposalIds.length === 0) return;
  const existing = new Set(localGovernanceCards.value.map((card) => card.id));
  const proposals = await Promise.allSettled(proposalIds.map((id) => getLaputaProposal(id)));
  const cards = proposals
    .filter((result): result is PromiseFulfilledResult<EvolutionProposal> => result.status === 'fulfilled')
    .map((result) => toProposalCard(result.value))
    .filter((card) => !existing.has(card.id));
  if (cards.length > 0) {
    localGovernanceCards.value = [...localGovernanceCards.value, ...cards];
    scrollToBottom();
  }
};

const pollAutoDreamRun = (runId: string, cardId: string, attempt = 0) => {
  const timer = setTimeout(async () => {
    autoDreamPollTimers.delete(timer);
    try {
      const run = await getAutoDreamRunStatus(runId);
      replaceGovernanceCard(cardId, toRunCard(run));
      scrollToBottom();
      if (run.state === 'pending' || run.state === 'running') {
        pollAutoDreamRun(runId, cardId, attempt + 1);
      } else {
        await appendProposalCards(run);
      }
    } catch (error) {
      replaceGovernanceCard(cardId, {
        kind: 'autodream_run',
        id: runId,
        state: 'unavailable',
        trigger: 'manual',
        summary: t('chatGovernance.backendUnavailable'),
        proposal_ids: [],
        error: normalizeError(error),
        source_run_id: runId,
      });
      scrollToBottom();
    }
  }, Math.min(1000 + attempt * 500, 5000));
  autoDreamPollTimers.add(timer);
};

const handleAutoDreamTrigger = async () => {
  if (autoDreamTriggering.value) return;
  autoDreamTriggering.value = true;
  const pendingId = `autodream-local-${Date.now()}`;
  const pendingCard: ChatGovernanceCardModel = {
    kind: 'autodream_run',
    id: pendingId,
    state: 'running',
    trigger: 'manual',
    summary: t('chatGovernance.triggerStarted'),
    proposal_ids: [],
    created_at: new Date().toISOString(),
  };
  localGovernanceCards.value = [...localGovernanceCards.value, pendingCard];
  scrollToBottom();

  try {
    const run = await triggerAutoDream('manual');
    replaceGovernanceCard(pendingId, toRunCard(run));
    scrollToBottom();
    if (run.state === 'pending' || run.state === 'running') {
      pollAutoDreamRun(run.id, run.id);
    } else {
      await appendProposalCards(run);
    }
  } catch (error) {
    localGovernanceCards.value = localGovernanceCards.value.map((card) =>
      card.id === pendingId
        ? {
            ...card,
            state: 'unavailable',
            summary: t('chatGovernance.backendUnavailable'),
            error: normalizeError(error),
          }
        : card,
    );
    scrollToBottom();
  } finally {
    autoDreamTriggering.value = false;
  }
};

const handleKeyDown = (e: KeyboardEvent) => {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
};

// 自动调整输入框高度
const adjustInputHeight = () => {
  if (inputRef.value) {
    inputRef.value.style.height = 'auto';
    const newHeight = Math.min(inputRef.value.scrollHeight, 120);
    inputRef.value.style.height = `${newHeight}px`;
    inputHeight.value = newHeight;
  }
};

// 获取动态 placeholder
const getPlaceholder = computed(() => {
  if (execMode.value === 'plan') {
    return t('chat.planMode') + ' · ' + t('chat.placeholder');
  }
  if (execMode.value === 'ask') {
    return t('chat.askMode');
  }
  return t('chat.placeholder');
});

/** Prefer explicit pending prop; fall back to active runtime awaiting approval. */
const approvalPlan = computed(() => {
  if (props.pendingApprovalPlan) return props.pendingApprovalPlan;
  const active = props.activePlanRuntime;
  if (!active) return null;
  if (active.phase === 'AwaitingApproval' || active.status === 'AwaitingApproval') {
    return active;
  }
  return null;
});

const planProgressText = computed(() => {
  const plan = props.executingPlan ?? props.activePlanRuntime;
  if (!plan) return '';
  const active = filterActivePlanTodos(plan.todos);
  if (active.length === 0) return plan.todos.length === 0 ? '无执行清单' : '暂无活动 TODO';
  return `${active.length} 项活动 TODO`;
});

const activePlanTodos = computed(() =>
  filterActivePlanTodos(props.activePlanRuntime?.todos ?? []),
);

const activePlanTodo = computed(() => {
  const todos = activePlanTodos.value;
  return todos.find((todo) => todo.status === 'InProgress') ?? todos.find((todo) => todo.status !== 'Completed') ?? null;
});

const activePlanTodoExpanded = ref(false);
const planTasksOpen = ref(false);
const selectedTodoId = ref<string | null>(null);

const selectedPlanTodo = computed(() =>
  props.activePlanRuntime?.todos.find((todo) => todo.id === selectedTodoId.value) ?? null,
);

const openPlanTasks = () => {
  if (props.activePlanRuntime) planTasksOpen.value = !planTasksOpen.value;
};

const openTodoStatus = (todoId: string) => {
  selectedTodoId.value = todoId;
  planTasksOpen.value = false;
};

const closeTodoStatus = () => {
  selectedTodoId.value = null;
};

// 模式菜单选项
const modeOptions = [
  { value: 'agent', label: 'chat.agentMode', icon: Zap, desc: 'chat.agentModeDesc' },
  { value: 'plan', label: 'chat.planMode', icon: Settings2, desc: 'chat.planModeDesc' },
  { value: 'ask', label: 'chat.askMode', icon: Brain, desc: 'chat.askModeDesc' },
];

// 权限模式选项
const permissionOptions = [
  { value: 'cautious', label: 'chat.permissionCautious', icon: Shield, desc: 'chat.permissionCautiousDesc' },
  { value: 'smart', label: 'chat.permissionSmart', icon: Sparkles, desc: 'chat.permissionSmartDesc' },
  { value: 'trusted', label: 'chat.permissionTrusted', icon: CheckCircle, desc: 'chat.permissionTrustedDesc' },
];

const getEmotionEmoji = (emotion?: string) => {
  const emotions: Record<string, string> = {
    happy: '😊',
    sad: '😢',
    clingy: '🥺',
    jealous: '😤',
    angry: '😠',
    normal: '🙂',
    // Fallback if needed
  };
  return emotions[emotion || 'normal'] || '🙂';
};

// 消息操作：复制
const copyMessage = async (content: string) => {
  try {
    await navigator.clipboard.writeText(content);
  } catch (err) {
    console.error('Failed to copy message:', err);
  }
};

// 格式化时间戳
const formatTime = (timestamp?: number) => {
  if (!timestamp) return '';
  return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
};

// ── Card rendering helpers ────────────────────────────────
/** Parse tool message content as card data, return null if invalid */
const parseCard = (content: string): Record<string, unknown> | null => {
  if (!content) return null;
  try {
    const parsed = JSON.parse(content);
    return typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed) ? parsed : null;
  } catch {
    return null;
  }
};

const asGovernanceCard = (card: Record<string, unknown> | null): ChatGovernanceCardModel | null => {
  if (!card || (card.kind !== 'autodream_run' && card.kind !== 'evolution_proposal')) {
    return null;
  }
  return card as unknown as ChatGovernanceCardModel;
};

const isGovernanceCard = (card: Record<string, unknown> | null) => {
  return card?.kind === 'autodream_run' || card?.kind === 'evolution_proposal';
};

const emitOpenEvolution = (payload: ChatGovernanceDeepLink) => {
  emit('open-evolution', payload);
};

/** Cache for parseCard results, keyed by message id. Cleared on message reset. */
const cardCache = new Map<string, Record<string, unknown> | null>();

/** Cached parseCard — avoids double-parse per card render. */
const getCachedCard = (messageId: string, content: string): Record<string, unknown> | null => {
  if (cardCache.has(messageId)) return cardCache.get(messageId)!;
  const result = parseCard(content);
  cardCache.set(messageId, result);
  return result;
};

/** Handle DecisionCard approve/reject action */
const onCardAction = (payload: { id: string; decision: 'approved' | 'rejected' }) => {
  console.log('[ChatView] card action:', payload);
};

/** Handle TodoCard item check toggle */
const onCardCheck = (payload: { id: string; item_id: string; status: 'pending' | 'done' }) => {
  console.log('[ChatView] card check:', payload);
};

/** Handle ApprovalBanner allow/reject response */
const onApprovalRespond = (payload: { request_id: string; decision: 'allow' | 'reject' }) => {
  console.log('[ChatView] approval respond:', payload);
};
</script>

<template>
  <div class="chat-shell flex flex-row h-full relative overflow-hidden" :class="[`theme-${themeMode || 'love'}`, { 'conv-sidebar-open': convSidebarOpen }]">
    <!-- Main Chat Area -->
    <div class="chat-main flex flex-col flex-1 min-w-0">
      <!-- Sidebar Toggle Button (top-right of chat area) -->
      <button
        @click="convSidebarOpen = !convSidebarOpen"
        class="conv-sidebar-toggle"
        :title="convSidebarOpen ? t('convSidebar.close') : t('convSidebar.open')"
      >
        <Clock v-if="convSidebarOpen" :size="18" />
        <Clock v-else :size="18" />
      </button>
      <!-- Sakura Effect -->
      <div v-if="themeMode === 'love'" class="chat-sakura">
        <span
          v-for="(s, i) in sakura"
          :key="i"
          class="sakura-petal"
          :style="{
            left: s.left,
            top: s.top,
            width: `${s.size}px`,
            height: `${s.size}px`,
            opacity: s.opacity,
            animationDelay: `${s.delay}s`,
          }"
        />
      </div>

      <!-- Messages List -->
      <div class="chat-list flex-1 overflow-y-auto p-4 space-y-4 scrollbar-thin z-10">
      <div v-if="messages.length === 0" class="flex flex-col items-center justify-center h-full text-gray-400 space-y-4">
        <div class="chat-empty-icon w-20 h-20 rounded-full flex items-center justify-center text-4xl animate-pulse">
          💕
        </div>
        <p class="text-lg">{{ t('chat.start') }}</p>
      </div>

      <div
        v-for="msg in messages"
        :key="msg.id"
        class="flex mb-4"
        :class="msg.role === 'user' ? 'justify-end' : 'justify-start'"
      >
        <div class="flex max-w-[85%] items-start space-x-2" :class="msg.role === 'user' ? 'flex-row-reverse space-x-reverse' : 'flex-row'">
          <!-- Avatar -->
          <div
            v-if="msg.role !== 'user' && msg.role !== 'tool'"
            class="chat-avatar w-9 h-9 rounded-md flex items-center justify-center text-xl flex-shrink-0"
          >
            {{ getEmotionEmoji(msg.emotion) }}
          </div>
          <div
            v-else-if="msg.role === 'tool'"
            class="w-9 h-9 rounded-md flex items-center justify-center flex-shrink-0 bg-gray-100 text-gray-500 border border-gray-200"
          >
            <Wrench :size="16" />
          </div>
          <div
            v-else
            class="chat-avatar-me w-9 h-9 rounded-md flex items-center justify-center text-xs flex-shrink-0"
          >
            Me
          </div>

          <!-- Bubble -->
          <div
            class="flex flex-col min-w-0 max-w-full"
            :class="msg.role === 'user' ? 'items-end' : 'items-start'"
          >
            <!-- Tool Message -->
            <template v-if="msg.role === 'tool'">
              <!-- Card rendering: plan_create / todo_write / approval_request -->
              <template v-if="isGovernanceCard(getCachedCard(msg.id, msg.content))">
                <div class="min-w-0">
                  <ChatGovernanceCard
                    :card="asGovernanceCard(getCachedCard(msg.id, msg.content))!"
                    @open-evolution="emitOpenEvolution"
                  />
                </div>
              </template>
              <template v-else-if="msg.toolName === 'plan_create' && getCachedCard(msg.id, msg.content)">
                <div class="min-w-0">
                  <DecisionCard
                    :card="(getCachedCard(msg.id, msg.content) as unknown as UiCard)"
                    @action="onCardAction"
                  />
                </div>
              </template>
              <template v-else-if="msg.toolName === 'todo_write' && getCachedCard(msg.id, msg.content)">
                <div class="min-w-0">
                  <TodoCard
                    :card="(getCachedCard(msg.id, msg.content) as unknown as UiCard)"
                    @check="onCardCheck"
                  />
                </div>
              </template>
              <template v-else-if="msg.toolName === 'update_plan' && getCachedCard(msg.id, msg.content)">
                <div class="min-w-0">
                  <TodoCard
                    :card="(getCachedCard(msg.id, msg.content) as unknown as UiCard)"
                  />
                </div>
              </template>
              <template v-else-if="msg.toolName === 'approval_request' && getCachedCard(msg.id, msg.content)">
                <div class="min-w-0">
                  <ApprovalBanner
                    :request="(getCachedCard(msg.id, msg.content) as unknown as ApprovalRequest)"
                    @respond="onApprovalRespond"
                  />
                </div>
              </template>
              <!-- Default tool output rendering -->
              <div
                v-else
                class="rounded-lg border text-sm overflow-hidden bg-white"
                :class="{
                  'border-gray-200': msg.toolStatus === 'running',
                  'border-green-200 bg-green-50/50': msg.toolStatus === 'success',
                  'border-red-200 bg-red-50/50': msg.toolStatus === 'error'
                }"
              >
                <!-- Tool Header -->
                <div class="px-3 py-2 flex items-center space-x-2">
                  <div v-if="msg.toolStatus === 'running'" class="animate-spin text-gray-400">
                    <Loader2 :size="14" />
                  </div>
                  <div v-else-if="msg.toolStatus === 'success'" class="text-green-500">
                    <CheckCircle2 :size="14" />
                  </div>
                  <div v-else class="text-red-500">
                    <XCircle :size="14" />
                  </div>

                  <span class="font-medium" :class="{
                    'text-gray-600': msg.toolStatus === 'running',
                    'text-green-700': msg.toolStatus === 'success',
                    'text-red-700': msg.toolStatus === 'error'
                  }">
                    {{ msg.toolStatus === 'running' ? t('chat.toolRunning') : (msg.toolStatus === 'success' ? t('chat.toolSuccess') : t('chat.toolFailed')) }}
                  </span>

                  <span v-if="msg.toolName" class="text-xs px-1.5 py-0.5 rounded bg-gray-100 text-gray-500 border border-gray-200">
                    {{ msg.toolName }}
                  </span>
                </div>

                <!-- Tool Details Toggle -->
                <div
                  v-if="msg.toolStatus !== 'running' && msg.toolResult"
                  class="px-3 pb-1 text-xs text-gray-600 break-all whitespace-pre-wrap"
                >
                  {{ msg.toolResult.length > 160 ? `${msg.toolResult.slice(0, 160)}...` : msg.toolResult }}
                </div>
                <div v-if="msg.toolStatus !== 'running'" class="px-3 pb-2 flex justify-end">
                  <button
                    @click="toggleTool(msg.id)"
                    class="text-[10px] flex items-center space-x-1 text-gray-400 hover:text-gray-600 transition-colors"
                  >
                    <span>{{ expandedTools[msg.id] ? t('chat.hideDetails') : t('chat.viewDetails') }}</span>
                    <component :is="expandedTools[msg.id] ? ChevronDown : ChevronRight" :size="12" />
                  </button>
                </div>

                <!-- Tool Details Content -->
                <div v-if="expandedTools[msg.id]" class="border-t border-gray-100 bg-gray-50/50 p-3 text-xs space-y-2">
                  <div v-if="msg.toolCallId">
                    <div class="font-semibold text-gray-500 mb-1">tool_call_id</div>
                    <div class="bg-white border border-gray-200 rounded p-2 font-mono text-gray-600 break-all whitespace-pre-wrap">{{ msg.toolCallId }}</div>
                  </div>
                  <div>
                    <div class="font-semibold text-gray-500 mb-1">{{ t('chat.inputArgs') }}</div>
                    <div class="bg-gray-100 rounded p-2 font-mono text-gray-600 break-all whitespace-pre-wrap">{{ msg.toolArgs }}</div>
                  </div>
                  <div v-if="msg.toolResult">
                    <div class="font-semibold text-gray-500 mb-1">{{ t('chat.execResult') }}</div>
                    <div class="bg-white border border-gray-200 rounded p-2 font-mono text-gray-600 max-h-40 overflow-y-auto break-all whitespace-pre-wrap">{{ msg.toolResult }}</div>
                  </div>
                </div>

                <div v-if="hasRawMeta(msg)" class="border-t border-gray-100 bg-white/70 px-3 py-2">
                  <button
                    @click="toggleRawMeta(msg.id)"
                    class="text-[10px] flex items-center space-x-1 text-gray-400 hover:text-gray-600 transition-colors"
                  >
                    <span>{{ expandedRawMeta[msg.id] ? t('chat.hideRawMeta') : t('chat.viewRawMeta') }}</span>
                    <component :is="expandedRawMeta[msg.id] ? ChevronDown : ChevronRight" :size="12" />
                  </button>
                  <div
                    v-if="expandedRawMeta[msg.id]"
                    class="mt-2 bg-gray-50 border border-gray-200 rounded p-2 font-mono text-[11px] text-gray-600 max-h-52 overflow-y-auto whitespace-pre-wrap break-all"
                  >
                    {{ renderRawMeta(msg) }}
                  </div>
                </div>
              </div>
            </template>

            <!-- Normal Message -->
            <div
              v-else
              class="chat-bubble relative px-4 py-3 rounded-2xl text-sm leading-relaxed break-words"
              :class="[
                msg.role === 'user' ? 'chat-bubble-user' : 'chat-bubble-assistant',
                { 'chat-bubble-has-reasoning': Boolean(msg.reasoning) },
              ]"
            >
              <!-- Reasoning Block -->
              <div v-if="msg.reasoning" class="streaming-reasoning-section">
                <ThinkingBlock
                  :content="msg.reasoning"
                  :thinking-ms="0"
                />
                <div
                  v-if="!msg.content && msg.isStreaming"
                  class="streaming-reasoning-status"
                  aria-live="polite"
                >
                  <span>{{ t('chat.thinking') }}</span>
                  <div class="streaming-dots" aria-label="Loading">
                    <i />
                    <i />
                    <i />
                  </div>
                </div>
              </div>

              <div v-if="hasRawMeta(msg)" class="mb-2 rounded border border-gray-200/50 bg-white/40 overflow-hidden">
                <div
                  @click="toggleRawMeta(msg.id)"
                  class="flex items-center justify-between px-2 py-1.5 cursor-pointer hover:bg-black/5 transition-colors select-none"
                >
                  <span class="text-xs text-gray-500">{{ t('chat.rawMeta') }}</span>
                  <component :is="expandedRawMeta[msg.id] ? ChevronDown : ChevronRight" :size="14" class="text-gray-400" />
                </div>
                <div v-if="expandedRawMeta[msg.id]" class="px-3 py-2 border-t border-gray-100/50 bg-gray-50/30 text-xs text-gray-600">
                  <div class="font-mono whitespace-pre-wrap break-all">{{ renderRawMeta(msg) }}</div>
                </div>
              </div>
              
              <!-- Content or Loading -->
              <div
                v-if="!msg.content && !msg.reasoning && msg.role === 'agent' && msg.isStreaming"
                class="streaming-dots streaming-dots-only"
                aria-label="Loading"
              >
                 <i />
                 <i />
                 <i />
              </div>
              <div v-else>
                <!-- Agent/system text: demux <proposed_plan> into a plan message block -->
                <AgentMessageBody
                  v-if="msg.role === 'agent' || msg.role === 'system'"
                  :content="msg.content"
                />
                <div v-else class="markdown-body" v-html="md.render(msg.content)"></div>
                <!-- 流式光标：内容存在且正在流式输出时显示 -->
                <span v-if="msg.content && msg.role === 'agent' && msg.isStreaming" class="streaming-cursor"></span>
              </div>
            </div>

            <!-- Message Actions: 时间戳 + 操作按钮 -->
            <div
              v-if="msg.role !== 'tool'"
              class="msg-actions"
              :class="msg.role === 'user' ? 'justify-end' : 'justify-start'"
            >
              <span class="text-[10px] text-gray-400">{{ formatTime(msg.timestamp) }}</span>
              <!-- 复制按钮（启用） -->
              <button
                class="msg-action-btn"
                @click="copyMessage(msg.content)"
                :title="t('chat.copy')"
              >
                <Copy :size="12" />
              </button>
              <!-- 编辑按钮（用户消息，disabled占位） -->
              <button
                v-if="msg.role === 'user'"
                class="msg-action-btn"
                disabled
                :title="t('chat.edit') + ' (' + t('chat.pending') + ')'"
              >
                <Edit :size="12" />
              </button>
              <!-- 重生成按钮（助手消息） -->
              <button
                v-if="msg.role === 'agent'"
                class="msg-action-btn"
                :disabled="isTyping"
                :title="t('chat.regenerate')"
                @click="emit('regenerate', msg.id)"
              >
                <RefreshCw :size="12" />
              </button>
              <!-- 回退按钮（disabled占位） -->
              <button
                class="msg-action-btn"
                disabled
                :title="t('chat.rewind') + ' (' + t('chat.pending') + ')'"
              >
                <Rewind :size="12" />
              </button>
              <!-- 分叉按钮（disabled占位） -->
              <button
                class="msg-action-btn"
                disabled
                :title="t('chat.fork') + ' (' + t('chat.pending') + ')'"
              >
                <GitFork :size="12" />
              </button>
            </div>

            <!-- Tool消息时间戳 -->
            <span
              v-else
              class="text-[10px] text-gray-400 mt-1 text-left"
            >
              {{ formatTime(msg.timestamp) }}
            </span>
          </div>
        </div>
      </div>

      <div
        v-for="card in localGovernanceCards"
        :key="card.id"
        class="flex mb-4 justify-start"
      >
        <div class="flex max-w-[85%] items-start space-x-2">
          <div class="w-9 h-9 rounded-md flex items-center justify-center flex-shrink-0 bg-blue-50 text-blue-600 border border-blue-100">
            <GitBranch :size="16" />
          </div>
          <div class="flex flex-col min-w-0 max-w-full">
            <ChatGovernanceCard :card="card" @open-evolution="emitOpenEvolution" />
            <span class="text-[10px] text-gray-400 mt-1 text-left">{{ formatTime(Date.now()) }}</span>
          </div>
        </div>
      </div>

      <PlanApprovalCard
        v-if="approvalPlan"
        :plan="approvalPlan"
        :approving="approvingPlan"
        @approve="handleApprovePlan"
        @revoke="emit('revoke-plan', $event)"
        @refresh="emit('refresh-plan')"
      />

      <!-- Typing Indicator -->
      <!-- Removed separate Typing Indicator as it is now integrated into the message bubble -->
      
      <div ref="messagesEndRef" />
    </div>

    <div
      v-if="activePlanRuntime && !approvalPlan"
      class="active-plan-todo-panel"
    >
      <div class="active-plan-todo-bar" role="button" tabindex="0" @click="openPlanTasks" @keydown.enter="openPlanTasks">
        <ClipboardList :size="16" class="active-plan-todo-icon" />
        <div class="active-plan-todo-content">
          <span class="active-plan-todo-plan">{{ activePlanRuntime.title }}</span>
          <span v-if="activePlanTodo" class="active-plan-todo-title">{{ activePlanTodo.title }}</span>
          <span v-else class="active-plan-todo-title">{{ activePlanRuntime.todos.length === 0 ? '无执行清单，按批准计划执行' : activePlanRuntime.phase }}</span>
        </div>
        <span class="active-plan-todo-progress">{{ planProgressText }}</span>
        <Loader2 v-if="activePlanTodo?.status === 'InProgress'" :size="15" class="text-amber-500 animate-spin" />
        <button
          type="button"
          class="active-plan-todo-toggle"
          :title="activePlanTodoExpanded ? '收起 TODO 详情' : '展开 TODO 详情'"
          :aria-label="activePlanTodoExpanded ? '收起 TODO 详情' : '展开 TODO 详情'"
          @click="activePlanTodoExpanded = !activePlanTodoExpanded"
        >
          <ChevronDown v-if="activePlanTodoExpanded" :size="15" />
          <ChevronRight v-else :size="15" />
        </button>
      </div>
      <div v-if="planTasksOpen" class="active-plan-task-list">
        <div class="active-plan-task-list-title">{{ activePlanRuntime.title }} · 任务</div>
        <button
          v-for="todo in activePlanTodos"
          :key="todo.id"
          type="button"
          class="active-plan-task-item"
          @click.stop="openTodoStatus(todo.id)"
        >
          <CheckCircle2 v-if="todo.status === 'Completed'" :size="14" class="text-emerald-500" />
          <Loader2 v-else-if="todo.status === 'InProgress'" :size="14" class="text-amber-500 animate-spin" />
          <Clock v-else :size="14" class="text-gray-400" />
          <span>{{ todo.title }}</span>
          <ChevronRight :size="14" class="active-plan-task-chevron" />
        </button>
      </div>
      <div v-if="activePlanTodoExpanded" class="active-plan-todo-details">
        <div class="active-plan-todo-details-title">{{ activePlanRuntime.title }}</div>
        <div v-if="activePlanRuntime.steps.length > 0" class="active-plan-todo-details-list">
          <div v-for="step in activePlanRuntime.steps" :key="step.id" class="active-plan-todo-detail-item">
            <CheckCircle2 v-if="step.status === 'Completed'" :size="13" class="text-emerald-500" />
            <Loader2 v-else-if="step.status === 'InProgress'" :size="13" class="text-amber-500 animate-spin" />
            <Clock v-else :size="13" class="text-gray-400" />
            <span>{{ step.ordinal + 1 }}. {{ step.title }}</span>
          </div>
        </div>
        <div v-else class="active-plan-todo-details-list">
          <div v-for="todo in activePlanRuntime.todos" :key="todo.id" class="active-plan-todo-detail-item">
            <CheckCircle2 v-if="todo.status === 'Completed'" :size="13" class="text-emerald-500" />
            <Loader2 v-else-if="todo.status === 'InProgress'" :size="13" class="text-amber-500 animate-spin" />
            <Clock v-else :size="13" class="text-gray-400" />
            <span>{{ todo.title }}</span>
          </div>
        </div>
      </div>
    </div>

    <div v-if="selectedPlanTodo" class="todo-status-overlay" @click.self="closeTodoStatus">
      <section class="todo-status-dialog" role="dialog" aria-modal="true" aria-label="任务状态">
        <header class="todo-status-header">
          <div>
            <div class="todo-status-eyebrow">{{ activePlanRuntime?.title }}</div>
            <h2>{{ selectedPlanTodo.title }}</h2>
          </div>
          <button type="button" class="todo-status-close" title="关闭" aria-label="关闭" @click="closeTodoStatus">
            <X :size="18" />
          </button>
        </header>
        <div class="todo-status-body">
          <div class="todo-status-row"><span>状态</span><strong>{{ selectedPlanTodo.status }}</strong></div>
          <div class="todo-status-row"><span>优先级</span><strong>{{ selectedPlanTodo.priority }}</strong></div>
          <div v-if="selectedPlanTodo.detail" class="todo-status-section"><span>任务说明</span><p>{{ selectedPlanTodo.detail }}</p></div>
          <div v-if="selectedPlanTodo.block_reason" class="todo-status-section todo-status-blocked"><span>阻塞原因</span><p>{{ selectedPlanTodo.block_reason }}</p></div>
          <div v-if="selectedPlanTodo.evidence_ref" class="todo-status-section"><span>验证证据</span><p>{{ selectedPlanTodo.evidence_ref }}</p></div>
        </div>
      </section>
    </div>

    <!-- Input Area - Cursor/OpenAkita 风格 -->
    <div class="chat-input-bar border-t z-20">
      <div class="chat-input-container">
        <!-- 附件预览区 -->
        <div v-if="attachments.length > 0 || uploadingPastes" class="flex flex-wrap gap-2 px-3 pt-2">
          <div
            v-for="(att, idx) in attachments"
            :key="att.file_id"
            class="flex items-center gap-1 bg-black/5 dark:bg-white/10 rounded-md px-2 py-1 text-xs"
          >
            <Paperclip :size="12" class="shrink-0 opacity-60" />
            <span class="truncate max-w-[100px]">{{ att.filename }}</span>
            <button @click="removeAttachment(idx)" class="shrink-0 opacity-60 hover:opacity-100" :title="t('chat.removeAttachment')">
              <X :size="12" />
            </button>
          </div>
          <div v-if="uploadingPastes" class="flex items-center gap-1 bg-black/5 dark:bg-white/10 rounded-md px-2 py-1 text-xs">
            <Loader2 :size="12" class="animate-spin opacity-60" />
            <span class="truncate max-w-[100px]">{{ t('chat.pasting') || 'Pasting...' }}</span>
          </div>
        </div>
        <!-- 顶部工具栏 -->
        <div class="chat-input-toolbar">
          <!-- 执行模式选择 -->
          <div class="relative">
            <button 
              @click="showModeMenu = !showModeMenu"
              class="toolbar-btn mode-selector"
            >
              <Zap v-if="execMode === 'agent'" :size="14" />
              <Settings2 v-else-if="execMode === 'plan'" :size="14" />
              <Brain v-else :size="14" />
              <span>{{ t(execMode === 'agent' ? 'chat.agentMode' : execMode === 'plan' ? 'chat.planMode' : 'chat.askMode') }}</span>
              <ChevronDown :size="12" />
            </button>
            <!-- 模式下拉菜单 -->
            <div v-if="showModeMenu" class="mode-menu">
              <div 
                v-for="mode in modeOptions" 
                :key="mode.value"
                @click="execMode = mode.value as any; showModeMenu = false"
                class="mode-menu-item"
                :class="{ active: execMode === mode.value }"
              >
                <component :is="mode.icon" :size="16" />
                <div class="mode-menu-text">
                  <div class="mode-label">{{ t(mode.label) }}</div>
                  <div class="mode-desc">{{ t(mode.desc) }}</div>
                </div>
                <CheckCircle2 v-if="execMode === mode.value" :size="14" />
              </div>
            </div>
          </div>

          <button class="toolbar-btn" :title="uploading ? t('chat.uploading') : t('chat.attachFile')" @click="fileInputRef?.click()" :disabled="uploading">
            <Loader2 v-if="uploading" :size="14" class="animate-spin" />
            <Paperclip v-else :size="14" />
          </button>
          <input type="file" ref="fileInputRef" @change="handleFileSelect" class="hidden" multiple accept="image/*,.pdf,.txt,.md,.json,.csv,.zip,.tar.gz" />

          <!-- 语音按钮 -->
          <!-- 思考模式选择 -->
          <ThinkingToggle v-model="thinkingMode" />

          <button
            class="toolbar-btn"
            :title="autoDreamTriggering ? t('chatGovernance.triggering') : t('chatGovernance.triggerManual')"
            :disabled="autoDreamTriggering"
            @click="handleAutoDreamTrigger"
          >
            <Loader2 v-if="autoDreamTriggering" :size="14" class="animate-spin" />
            <GitBranch v-else :size="14" />
          </button>

          <!-- 桌面宠物按钮 -->
          <button
            class="toolbar-btn"
            :title="t('chat.openPet')"
            @click="invoke('open_desktop_pet')"
          >
            <Cat :size="14" />
          </button>

          <!-- 权限模式选择 -->
          <div class="relative">
            <button 
              @click="showPermissionMenu = !showPermissionMenu"
              class="toolbar-btn permission-btn"
            >
              <component :is="permissionOptions.find(p => p.value === permissionMode)?.icon || Sparkles" :size="14" />
              <span>{{ t(permissionOptions.find(p => p.value === permissionMode)?.label || 'chat.permissionSmart') }}</span>
              <ChevronDown :size="12" />
            </button>
            <!-- 权限模式下拉菜单 -->
            <div v-if="showPermissionMenu" class="mode-menu">
              <div 
                v-for="perm in permissionOptions" 
                :key="perm.value"
                @click="permissionMode = perm.value as any; showPermissionMenu = false"
                class="mode-menu-item"
                :class="{ active: permissionMode === perm.value }"
              >
                <component :is="perm.icon" :size="16" />
                <div class="mode-menu-text">
                  <div class="mode-label">{{ t(perm.label) }}</div>
                  <div class="mode-desc">{{ t(perm.desc) }}</div>
                </div>
                <CheckCircle v-if="permissionMode === perm.value" :size="14" />
              </div>
            </div>
          </div>
        </div>

        <!-- 主输入区 -->
        <div class="chat-input-main">
          <textarea
            ref="inputRef"
            v-model="input"
            @input="adjustInputHeight"
            @keydown="handleKeyDown"
            @paste="handlePaste"
            :placeholder="getPlaceholder"
            class="chat-textarea"
            rows="1"
          />
        </div>

        <!-- 底部操作栏 -->
        <div class="chat-input-footer">
          <!-- 上下文使用指示器 -->
          <div class="context-usage" :title="contextUsageTitle">
            <svg class="context-ring" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="9" fill="none" stroke="#e5e7eb" stroke-width="2"/>
              <circle
                cx="12"
                cy="12"
                r="9"
                fill="none"
                :stroke="contextRingColor"
                stroke-width="2"
                stroke-dasharray="56.5"
                :stroke-dashoffset="contextRingDashoffset"
                stroke-linecap="round"
                transform="rotate(-90 12 12)"
              />
            </svg>
            <span class="context-text">{{ contextUsagePercent }}%</span>
          </div>

          <!-- 右侧按钮组 -->
          <div class="footer-actions">
            <!-- 新建会话按钮 -->
            <button
              @click="handleClear"
              class="input-action-btn left"
              :title="t('chat.newSession')"
            >
              <Plus :size="18" />
            </button>

            <button
              class="input-action-btn"
              :class="{ recording: isRecording }"
              :title="t('chat.voice')"
              @click="isRecording = !isRecording"
            >
              <Mic :size="18" />
            </button>

            <!-- 发送/停止按钮 -->
            <button
              v-if="isTyping"
              @click="handleStop"
              class="input-action-btn send stop-btn"
              :title="t('chat.stop')"
            >
              <Square :size="18" />
            </button>
            <button
              v-else
              @click="handleSend"
              :disabled="!input.trim() && attachments.length === 0"
              class="input-action-btn send"
              :class="{ disabled: !input.trim() && attachments.length === 0 }"
              :title="t('chat.send')"
            >
              <Send :size="18" />
            </button>
          </div>
        </div>
      </div>
    </div>
    </div> <!-- End chat-main -->

    <!-- Conversation Sidebar (Right Panel) -->
    <div
      v-if="convSidebarOpen && narrowLayout"
      class="conv-sidebar-scrim"
      @click="convSidebarOpen = false"
    />
    <ConversationSidebar
      v-if="convSidebarOpen"
      ref="convSidebarRef"
      :sessions="sessions || []"
      :active-session-key="activeSessionKey || ''"
      :theme-mode="themeMode || 'love'"
      @select="(key) => emit('select-session', key)"
      @delete="(key) => emit('delete-session', key)"
      @new="emit('new-session')"
      @toggle-pin="(key) => emit('toggle-pin', key)"
      @rename="(key, title) => emit('rename-session', key, title)"
      @refresh="emit('refresh-sessions')"
      @close="convSidebarOpen = false"
      class="conv-sidebar-wrapper"
      :class="{ 'conv-sidebar-wrapper--overlay': narrowLayout }"
    />
  </div>
</template>

<style scoped>
/* Sidebar Toggle Button */
.conv-sidebar-toggle {
  position: absolute;
  top: 12px;
  right: 12px;
  z-index: 50;
  padding: 8px;
  border-radius: 8px;
  border: 1px solid var(--line, #e5e7eb);
  background: var(--panel-solid, #ffffff);
  color: var(--text-muted, #9ca3af);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
}

/* When sidebar is open, move toggle button to the left edge of sidebar */
.conv-sidebar-open .conv-sidebar-toggle {
  right: 292px; /* 280px sidebar + 12px gap */
}

.conv-sidebar-toggle:hover {
  background: var(--nav-hover, rgba(0, 0, 0, 0.04));
  color: var(--text, #111827);
  border-color: var(--brand, #ec4899);
}

.active-plan-todo-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  min-width: 0;
  margin: 0 16px 8px;
  padding: 8px 10px;
  border: 1px solid rgba(245, 158, 11, .28);
  border-radius: 10px;
  background: rgba(255, 251, 235, .96);
  box-shadow: 0 4px 12px rgba(15, 23, 42, .08);
}
.active-plan-todo-panel { flex-shrink: 0; min-width: 0; margin: 0 16px 8px; }
.active-plan-todo-panel .active-plan-todo-bar { margin: 0; }
.active-plan-todo-icon { flex: 0 0 auto; color: #b45309; }
.active-plan-todo-content { display: flex; min-width: 0; flex: 1; align-items: baseline; gap: 8px; }
.active-plan-todo-plan { flex: 0 0 auto; max-width: 30%; overflow: hidden; color: #92400e; font-size: 11px; font-weight: 700; text-overflow: ellipsis; white-space: nowrap; }
.active-plan-todo-title { min-width: 0; overflow: hidden; color: #374151; font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
.active-plan-todo-progress { flex: 0 0 auto; color: #b45309; font-size: 11px; font-variant-numeric: tabular-nums; }
.active-plan-todo-toggle { display: flex; align-items: center; justify-content: center; flex: 0 0 auto; padding: 3px; border: 0; border-radius: 5px; color: #92400e; background: transparent; cursor: pointer; }
.active-plan-todo-toggle:hover { background: rgba(245, 158, 11, .14); }
.active-plan-todo-details { padding: 9px 12px 10px; border: 1px solid rgba(245, 158, 11, .24); border-top: 0; border-radius: 0 0 10px 10px; background: rgba(255, 251, 235, .96); }
.active-plan-todo-details-title { margin-bottom: 6px; color: #92400e; font-size: 11px; font-weight: 700; }
.active-plan-todo-details-list { display: grid; gap: 5px; }
.active-plan-todo-detail-item { display: flex; align-items: center; gap: 6px; min-width: 0; color: #374151; font-size: 11px; }
.active-plan-todo-detail-item span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.active-plan-task-list { display: grid; gap: 4px; margin-top: -1px; padding: 8px; border: 1px solid rgba(245, 158, 11, .24); border-top: 0; border-radius: 0 0 10px 10px; background: rgba(255, 251, 235, .96); }
.active-plan-task-list-title { padding: 2px 4px 5px; color: #92400e; font-size: 11px; font-weight: 700; }
.active-plan-task-item { display: flex; align-items: center; gap: 7px; min-width: 0; padding: 7px 6px; border: 0; border-radius: 7px; color: #374151; background: transparent; font-size: 11px; text-align: left; cursor: pointer; }
.active-plan-task-item:hover { background: rgba(245, 158, 11, .14); }
.active-plan-task-item span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.active-plan-task-chevron { margin-left: auto; flex: 0 0 auto; color: #b45309; }
.todo-status-overlay { position: absolute; inset: 0; z-index: 80; display: flex; align-items: center; justify-content: center; padding: 24px; background: rgba(15, 23, 42, .42); backdrop-filter: blur(3px); }
.todo-status-dialog { width: min(520px, 100%); max-height: min(80vh, 620px); overflow: auto; border: 1px solid var(--line, #e5e7eb); border-radius: 16px; background: var(--panel-solid, #fff); box-shadow: 0 24px 70px rgba(15, 23, 42, .25); }
.todo-status-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; padding: 18px 20px; border-bottom: 1px solid var(--line, #e5e7eb); }
.todo-status-eyebrow { margin-bottom: 5px; color: var(--text-muted, #9ca3af); font-size: 11px; }
.todo-status-header h2 { margin: 0; color: var(--text, #111827); font-size: 17px; font-weight: 700; }
.todo-status-close { display: flex; align-items: center; justify-content: center; padding: 5px; border: 0; border-radius: 7px; color: var(--text-muted, #9ca3af); background: transparent; cursor: pointer; }
.todo-status-close:hover { color: var(--text, #111827); background: var(--nav-hover, rgba(0, 0, 0, .06)); }
.todo-status-body { display: grid; gap: 14px; padding: 18px 20px 22px; color: var(--text, #111827); }
.todo-status-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding-bottom: 10px; border-bottom: 1px solid var(--line, #f1f5f9); font-size: 13px; }
.todo-status-row span, .todo-status-section > span { color: var(--text-muted, #6b7280); font-size: 12px; }
.todo-status-row strong { font-size: 13px; }
.todo-status-section p { margin: 5px 0 0; color: var(--text, #374151); font-size: 13px; line-height: 1.6; white-space: pre-wrap; }
.todo-status-blocked { padding: 10px 12px; border-radius: 9px; background: #fff7ed; }

/* Conversation Sidebar Wrapper */
.conv-sidebar-wrapper {
  flex-shrink: 0;
  height: 100%;
}

.conv-sidebar-wrapper--overlay {
  position: fixed;
  inset: 0 0 0 auto;
  z-index: 160;
  width: 280px;
  max-width: calc(100vw - 40px);
  box-shadow: -12px 0 28px rgba(15, 23, 42, 0.22);
}

.conv-sidebar-scrim {
  position: fixed;
  inset: 0;
  z-index: 150;
  background: rgba(15, 23, 42, 0.28);
}

/* Scoped styles if needed, but we rely on global tailwind classes mostly */
:deep(.markdown-body) {
  font-size: 0.875rem;
  line-height: 1.6;
}

.streaming-reasoning-section {
  display: grid;
  gap: 8px;
}

.streaming-reasoning-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 4px;
  color: var(--text-muted, #6b7280);
  font-size: 12px;
}

.streaming-dots {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-height: 12px;
}

.streaming-dots-only {
  padding: 6px 0;
}

.streaming-dots i {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: var(--text-muted, #9ca3af);
  animation: streaming-dot-bounce 1s infinite ease-in-out;
}

.streaming-dots i:nth-child(2) { animation-delay: .1s; }
.streaming-dots i:nth-child(3) { animation-delay: .2s; }

@keyframes streaming-dot-bounce {
  0%, 60%, 100% { transform: translateY(0); opacity: .45; }
  30% { transform: translateY(-3px); opacity: 1; }
}

:deep(.markdown-body p) {
  margin-bottom: 0.5em;
}

:deep(.markdown-body p:last-child) {
  margin-bottom: 0;
}

:deep(.markdown-body pre) {
  background-color: #1e1e1e;
  border-radius: 0.375rem;
  padding: 0.75rem;
  margin: 0.5rem 0;
  overflow-x: auto;
}

:deep(.markdown-body code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.85em;
  background-color: rgba(0, 0, 0, 0.1);
  padding: 0.2em 0.4em;
  border-radius: 0.25rem;
}

:deep(.markdown-body pre code) {
  background-color: transparent;
  padding: 0;
  color: #e5e7eb;
}

:deep(.markdown-body ul), :deep(.markdown-body ol) {
  padding-left: 1.5em;
  margin-bottom: 0.5em;
}

:deep(.markdown-body ul) {
  list-style-type: disc;
}

:deep(.markdown-body ol) {
  list-style-type: decimal;
}

:deep(.markdown-body blockquote) {
  border-left: 3px solid #e5e7eb;
  padding-left: 0.75rem;
  color: #6b7280;
  margin: 0.5rem 0;
}

:deep(.markdown-body a) {
  color: #3b82f6;
  text-decoration: underline;
}

:deep(.markdown-body a:hover) {
  color: #2563eb;
}
</style>
