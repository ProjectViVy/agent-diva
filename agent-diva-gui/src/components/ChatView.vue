<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted, onBeforeUnmount } from 'vue';
import { Send, Square, Plus, Wrench, ChevronDown, ChevronRight, CheckCircle, CheckCircle2, XCircle, Loader2, Brain, Copy, Edit, RefreshCw, Rewind, GitFork, Mic, Volume2, Settings2, Zap, Clock, Shield, ShieldCheck, Sparkles, ClipboardList, ImagePlus, X } from '@lucide/vue';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css'; // 使用 GitHub Dark 风格
import { useI18n } from 'vue-i18n';
import ConversationSidebar from './ConversationSidebar.vue';
import DecisionCard from './DecisionCard.vue';
import TodoCard from './TodoCard.vue';
import AskUserQuestionCard from './AskUserQuestionCard.vue';
export type { AskUserQuestionView } from './AskUserQuestionCard.vue';
import ThinkingBlock from './chat/ThinkingBlock.vue';
import ThinkingToggle from './chat/ThinkingToggle.vue';
import PlanApprovalCard from './planning/PlanApprovalCard.vue';
import AgentMessageBody from './planning/AgentMessageBody.vue';
import { activePlanTodos as filterActivePlanTodos } from './planning/planExecutionState';
import type {
  UiCard,
} from '../api/desktop';
import type { PermissionPreset, TurnAttachment } from '../api/vivy/contracts';
import {
  ChatAttachmentError,
  MAX_ATTACHMENTS,
  fileToTurnAttachment,
  framedRequestBytes,
  MAX_FRAME_BYTES,
} from '../state/chat-images';
import type { AskUserQuestionView } from './AskUserQuestionCard.vue';
import type { PlanRuntimeState } from '../api/planning';
import type { BudgetConfigShape } from '../types/toolsConfig';
import { budgetShapeFromCompaction, loadCompactionConfig } from '../api/settings';
import { initVoice, voiceController } from '../state/voice';
import { vivyCognitive } from '../state/vivy-cognitive';
import PersonaSetupGate from './persona-memory/PersonaSetupGate.vue';
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
  retryStatus?: { attempt: number; maxRetries: number; model?: string };
  stalled?: boolean;
  rawMeta?: Record<string, unknown>;
  fromHistory?: boolean;
  attachments?: string[];
  /** Owning run for run-folded segments (DN-6C replay fencing). */
  runId?: string;
}

export interface CompactionStatus {
  trigger: 'auto' | 'reactive';
  phase: 'started' | 'completed' | 'failed';
  summary?: string | null;
}

interface ToolResultRefV1 {
  version: 1;
  artifact_id: string;
  tool_call_id: string;
  tool_name: string;
  status: string;
  char_count: number;
  byte_count: number;
  sha256: string;
  preview: string;
  truncated: boolean;
  read_hint: string;
}

function parseToolResultRef(content?: string): ToolResultRefV1 | null {
  if (!content) return null;
  try {
    const value = JSON.parse(content) as Partial<ToolResultRefV1>;
    if (
      value.version !== 1 ||
      typeof value.artifact_id !== 'string' ||
      typeof value.tool_call_id !== 'string' ||
      typeof value.tool_name !== 'string' ||
      typeof value.status !== 'string' ||
      typeof value.char_count !== 'number' ||
      typeof value.byte_count !== 'number' ||
      typeof value.sha256 !== 'string' ||
      typeof value.preview !== 'string' ||
      typeof value.truncated !== 'boolean' ||
      typeof value.read_hint !== 'string'
    ) return null;
    return value as ToolResultRefV1;
  } catch {
    return null;
  }
}

const toolResultRef = (msg: Message) => parseToolResultRef(msg.toolResult || msg.content);
const toolResultPreview = (msg: Message) => {
  const reference = toolResultRef(msg);
  if (reference) return reference.preview;
  const result = msg.toolResult || '';
  return result.length > 160 ? `${result.slice(0, 160)}...` : result;
};

const expandedTools = ref<Record<string, boolean>>({});
const expandedReasoning = ref<Record<string, boolean>>({});
const expandedRawMeta = ref<Record<string, boolean>>({});

interface HistoryPrefs {
  cleanMode: boolean;
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
}

const defaultHistoryPrefs: HistoryPrefs = {
  cleanMode: false,
  autoExpandReasoning: true,
  autoExpandToolDetails: false,
  showRawMetaByDefault: false,
};

const toggleTool = (messageId: string) => {
  expandedTools.value[messageId] = !expandedTools.value[messageId];
};

const setReasoningExpanded = (messageId: string, expanded: boolean) => {
  expandedReasoning.value[messageId] = expanded;
};

const toggleRawMeta = (messageId: string) => {
  expandedRawMeta.value[messageId] = !expandedRawMeta.value[messageId];
};

const hasRawMeta = (msg: Message) => {
  return !!msg.rawMeta && Object.keys(msg.rawMeta).length > 0;
};

const toolDisplayName = (msg: Message) => msg.toolName?.trim() || t('app.unknownTool');

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
  activeSessionKey?: string;
  activePlanRuntime?: PlanRuntimeState | null;
  pendingApprovalPlan?: PlanRuntimeState | null;
  executingPlan?: PlanRuntimeState | null;
  approvingPlan?: boolean;
  planExecutionError?: string | null;
  approvalCenterOpen?: boolean;
  approvalPendingCount?: number;
  askUserQuestions?: AskUserQuestionView[];
  compactionStatus?: CompactionStatus | null;
}>();

const emit = defineEmits<{
  (e: 'send', content: string, attachments?: TurnAttachment[], mode?: 'agent' | 'plan' | 'ask', permissionMode?: PermissionPreset): void;
  (e: 'approve-plan', payload: { contextPolicy: 'retain' | 'compact' | 'clear' }): void;
  (e: 'start-goal', payload: { max_rounds: number }): void;
  (e: 'resume-plan'): void;
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
  (e: 'regenerate', messageId: string): void;
  (e: 'update:approval-center-open', open: boolean): void;
  (e: 'answer-ask-user', payload: { question_id: string; selected_index: number | null; other_text: string | null }): void;
  (e: 'cancel-ask-user', questionId: string): void;
}>();

const input = ref('');
const chatListRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLTextAreaElement | null>(null);

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
const PERMISSION_MODE_KEY = 'agent-diva.permissionMode';
const permissionMode = ref<'cautious' | 'smart' | 'trusted'>(
  (localStorage.getItem(PERMISSION_MODE_KEY) as 'cautious' | 'smart' | 'trusted') ?? 'smart',
);
watch(permissionMode, (mode) => {
  localStorage.setItem(PERMISSION_MODE_KEY, mode);
});
// const showAttachments = ref(false); // 预留
// DN-6C: voice state lives in the singleton controller — recording,
// transcribing and speaking read through it; the mic click toggles record
// and the same button interrupts playback while speaking.
const voice = voiceController();
/** DN-4D: persona setup gates the primary send while settings/setup stay reachable. */
const cog = vivyCognitive();
const personaSetupRequired = computed(() => cog.needsSetup.value);
const isRecording = computed(() => voice.state.value === 'recording');
const isTranscribing = computed(() => voice.state.value === 'transcribing');
const isSpeaking = computed(() => voice.state.value === 'speaking' || voice.playing.value);
const micTitle = computed(() => {
  if (isRecording.value) return t('chat.voiceTranscribing');
  if (isTranscribing.value) return t('chat.voiceTranscribing');
  if (isSpeaking.value) return t('chat.voiceSpeaking');
  return t('chat.voice');
});
const handleMicClick = () => {
  if (isRecording.value) {
    void voice.stopRecording();
  } else if (isTranscribing.value) {
    voice.cancelRecording();
  } else if (isSpeaking.value) {
    voice.stopSpeaking();
  } else {
    void voice.startRecording();
  }
};
// STT lands as an editable draft only — never an emit, never a send.
watch(
  () => voice.draft.value,
  (draft) => {
    if (!draft) return;
    input.value = draft;
    voice.draft.value = '';
    nextTick(() => {
      adjustInputHeight();
      inputRef.value?.focus();
    });
  },
);
watch(
  () => voice.error.value,
  (err) => {
    if (!err) return;
    attachmentError.value =
      err === 'voice_busy'
        ? t('chat.voiceErrorBusy')
        : err === 'native speech is unavailable in this context'
          ? t('chat.voiceErrorUnavailable')
          : t('chat.voiceErrorGeneric', { message: err });
    voice.error.value = null;
  },
);
// const recordingDuration = ref(0); // 预留
const thinkingMode = ref<'auto' | 'on' | 'off'>('auto');


const effectiveHistoryPrefs = computed<HistoryPrefs>(() => ({
  ...defaultHistoryPrefs,
  ...(props.historyPrefs ?? {}),
  ...(props.historyPrefs?.cleanMode
    ? {
        autoExpandReasoning: false,
        autoExpandToolDetails: false,
        showRawMetaByDefault: false,
      }
    : {}),
}));

const cleanMode = computed(() => effectiveHistoryPrefs.value.cleanMode);

const reconcileExpansionStates = () => {
  const prefs = effectiveHistoryPrefs.value;
  props.messages.forEach((msg) => {
    if (msg.reasoning) {
      expandedReasoning.value[msg.id] = prefs.autoExpandReasoning;
    }
    if (msg.role === 'tool') {
      expandedTools.value[msg.id] = prefs.autoExpandToolDetails;
    }
    if (hasRawMeta(msg)) {
      expandedRawMeta.value[msg.id] = prefs.showRawMetaByDefault;
    }
  });
};

const contextBudgetShape = ref<BudgetConfigShape | undefined>(undefined);
onMounted(() => {
  loadCompactionConfig()
    .then((cfg) => { contextBudgetShape.value = budgetShapeFromCompaction(cfg); })
    .catch(() => {});
});

const contextBudgetStatus = computed(() =>
  computeBudgetStatus(props.messages, contextBudgetShape.value)
);

const contextUsagePercent = computed(() =>
  Math.min(100, Math.max(0, budgetPressurePercent(contextBudgetStatus.value)))
);

const contextRingDashoffset = computed(() => {
  const circumference = 56.5;
  return circumference * (1 - contextUsagePercent.value / 100);
});

const contextRingColor = computed(() => {
  if (contextUsagePercent.value >= 80) return 'var(--warning)';
  if (contextUsagePercent.value >= 60) return 'var(--warning)';
  return 'var(--primary)';
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

const BOTTOM_FOLLOW_THRESHOLD_PX = 48;
const followsLatestMessage = ref(true);

const isChatListNearBottom = () => {
  const chatList = chatListRef.value;
  if (!chatList) return true;

  const distanceFromBottom = chatList.scrollHeight - chatList.scrollTop - chatList.clientHeight;
  return distanceFromBottom <= BOTTOM_FOLLOW_THRESHOLD_PX;
};

const handleChatScroll = () => {
  followsLatestMessage.value = isChatListNearBottom();
};

const scrollToBottom = (force = false) => {
  if (!force && !followsLatestMessage.value) return;
  followsLatestMessage.value = true;
  nextTick(() => {
    const chatList = chatListRef.value;
    if (!chatList) return;

    chatList.scrollTop = chatList.scrollHeight;
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
      expandedReasoning.value[msg.id] = effectiveHistoryPrefs.value.autoExpandReasoning;
    }
    if (expandedTools.value[msg.id] === undefined && msg.role === 'tool') {
      expandedTools.value[msg.id] = effectiveHistoryPrefs.value.autoExpandToolDetails;
    }
    if (expandedRawMeta.value[msg.id] === undefined && hasRawMeta(msg)) {
      expandedRawMeta.value[msg.id] = effectiveHistoryPrefs.value.showRawMetaByDefault;
    }
  });
  scrollToBottom();
}, { deep: true, immediate: true });

watch(effectiveHistoryPrefs, reconcileExpansionStates, { deep: true });

// 询问卡片 (askUserQuestions) 独立于 messages；仅在用户仍跟随最新内容时保持滚底。
watch(
  () => ({
    qLen: (props.askUserQuestions?.length) ?? 0,
    h: chatListRef.value?.scrollHeight ?? 0,
  }),
  () => { scrollToBottom(); },
);

watch(
  () => props.activeSessionKey,
  (activeSessionKey, previousSessionKey) => {
    if (activeSessionKey !== previousSessionKey) {
      scrollToBottom(true);
    }
  },
);

onMounted(() => {
  updateNarrowLayout();
  window.addEventListener('resize', updateNarrowLayout);
  scrollToBottom();
  inputRef.value?.focus();
  // DN-6C: init reads the native window generation + advances it via
  // context_set; browser mode stays inert through native_unavailable.
  void initVoice();
});

onBeforeUnmount(() => {
  window.removeEventListener('resize', updateNarrowLayout);
});

// DN-2A image attachments: picked files are decoded+validated once and
// held as wire-shape TurnAttachments until the send admits them.
const pendingImages = ref<TurnAttachment[]>([]);
const attachmentError = ref('');
const fileInputRef = ref<HTMLInputElement | null>(null);

const attachmentErrorText = (code: string): string => {
  switch (code) {
    case 'image_too_large': return t('chat.imageTooLarge');
    case 'too_many': return t('chat.imageCountMax', { max: MAX_ATTACHMENTS });
    case 'frame_too_large': return t('chat.sendFrameTooLarge');
    default: return t('chat.imageUnsupported');
  }
};

const triggerImagePicker = () => {
  fileInputRef.value?.click();
};

const handlePickImages = async (event: Event) => {
  const picker = event.target as HTMLInputElement;
  const files = Array.from(picker.files ?? []);
  picker.value = ''; // the same file can be picked again later
  attachmentError.value = '';
  for (const file of files) {
    if (pendingImages.value.length >= MAX_ATTACHMENTS) {
      attachmentError.value = t('chat.imageCountMax', { max: MAX_ATTACHMENTS });
      break;
    }
    try {
      pendingImages.value.push(await fileToTurnAttachment(file));
    } catch (e) {
      attachmentError.value =
        attachmentErrorText(e instanceof ChatAttachmentError ? e.code : 'unsupported_type');
    }
  }
};

const removeImage = (index: number) => {
  pendingImages.value.splice(index, 1);
  attachmentError.value = '';
};

/** Exact ABI frame check on the serialized call — the session id adds a
 * bounded tail, so 128 bytes of headroom keeps this pre-check equal to
 * the controller's authoritative measurement for any realistic id. */
const sendFrameTooLarge = (text: string, attachments: TurnAttachment[]): boolean =>
  framedRequestBytes('turn/start', {
    session_id: '',
    text,
    ...(attachments.length ? { attachments } : {}),
  }) + 128 > MAX_FRAME_BYTES;

const handleSend = () => {
  if (props.isTyping || personaSetupRequired.value) return;
  const content = input.value.trim();
  if (!content) return;
  const attachments = [...pendingImages.value];
  if (sendFrameTooLarge(content, attachments)) {
    attachmentError.value = t('chat.sendFrameTooLarge');
    return; // draft + picked files stay put; nothing is emitted
  }
  scrollToBottom(true);
  emit(
    'send',
    content,
    attachments.length ? attachments : undefined,
    execMode.value,
    permissionMode.value,
  );
  input.value = '';
  pendingImages.value = [];
  attachmentError.value = '';
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

const handleClear = () => {
  emit('clear');
};

const handleStop = () => {
  if (!props.isTyping) return;
  emit('stop');
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

const handleSelectSession = (sessionKey: string) => {
  followsLatestMessage.value = true;
  emit('select-session', sessionKey);
};

const handleNewSession = () => {
  followsLatestMessage.value = true;
  emit('new-session');
};

const copyArtifactReference = async (msg: Message) => {
  const reference = toolResultRef(msg);
  if (reference) await copyMessage(`artifact://${reference.artifact_id}`);
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




/** Cache for parseCard results, keyed by message id. Cleared on message reset. */
const cardCache = new Map<string, Record<string, unknown> | null>();

/** Cached parseCard — avoids double-parse per card render. */
const getCachedCard = (messageId: string, content: string): Record<string, unknown> | null => {
  if (cardCache.has(messageId)) return cardCache.get(messageId)!;
  const result = parseCard(content);
  cardCache.set(messageId, result);
  return result;
};

const hasInteractiveToolCard = (msg: Message) => {
  const card = getCachedCard(msg.id, msg.content);
  return (msg.toolName === 'plan_create' || msg.toolName === 'todo_write' || msg.toolName === 'update_plan') && !!card;
};

const isCleanProcessMessage = (msg: Message) => {
  if (msg.role === 'tool') return !hasInteractiveToolCard(msg);
  return msg.role === 'agent' && !msg.content.trim();
};

const cleanProcessGroup = (anchorIndex: number) => {
  const group: Message[] = [];
  for (let index = anchorIndex; index < props.messages.length; index += 1) {
    const message = props.messages[index];
    if (!isCleanProcessMessage(message)) break;
    group.push(message);
  }
  return group;
};

const isCleanProcessAnchor = (msg: Message, index: number) => {
  if (!cleanMode.value || !isCleanProcessMessage(msg)) return false;
  return index === 0 || !isCleanProcessMessage(props.messages[index - 1]);
};

const cleanProcessCurrentToolName = (anchorIndex: number) => {
  const group = cleanProcessGroup(anchorIndex);
  for (let index = group.length - 1; index >= 0; index -= 1) {
    const message = group[index];
    if (message.role === 'agent' && (message.isStreaming || message.isThinking)) return '';
    if (
      message.role === 'tool'
      && (message.toolStatus === 'running' || message.isStreaming || message.isThinking)
    ) {
      return toolDisplayName(message);
    }
  }
  return '';
};

const cleanProcessCurrentReasoning = (anchorIndex: number) => {
  const group = cleanProcessGroup(anchorIndex);
  for (let index = group.length - 1; index >= 0; index -= 1) {
    const message = group[index];
    if (message.role === 'tool' && message.toolStatus === 'running') return '';
    if (message.role === 'agent' && (message.isStreaming || message.isThinking)) {
      return (message.reasoning || '').replace(/\s+/g, ' ').trim();
    }
  }
  return '';
};

const cleanProcessIsActive = (anchorIndex: number) => cleanProcessGroup(anchorIndex)
  .some((message) => message.isStreaming || message.isThinking || message.toolStatus === 'running');

/** Handle DecisionCard approve/reject action */
const onCardAction = (payload: { id: string; decision: 'approved' | 'rejected' }) => {
  console.log('[ChatView] card action:', payload);
};

/** Handle TodoCard item check toggle */
const onCardCheck = (payload: { id: string; item_id: string; status: 'pending' | 'done' }) => {
  console.log('[ChatView] card check:', payload);
};

</script>

<template>
  <div class="chat-shell flex flex-row h-full relative overflow-hidden" :class="[`theme-${themeMode || 'love'}`, { 'conv-sidebar-open': convSidebarOpen }]">
    <!-- Main Chat Area -->
    <div class="chat-main flex flex-col flex-1 min-w-0">
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
      <div
        v-if="compactionStatus"
        class="compaction-status-line mx-4 mb-2 rounded-lg border border-border-strong bg-accent px-3 py-2 text-xs text-primary   "
        role="status"
      >
        <span v-if="compactionStatus.phase === 'started'">{{ t('chat.compactionRunning') }}</span>
        <span v-else-if="compactionStatus.phase === 'completed'">{{ t('chat.compactionCompleted') }}</span>
        <span v-else>
          {{ t('chat.compactionFailed') }}
          <span v-if="compactionStatus.summary">{{ ` ${compactionStatus.summary}` }}</span>
        </span>
      </div>
      <div
        ref="chatListRef"
        class="chat-list flex-1 overflow-y-auto p-4 space-y-4 scrollbar-thin z-10"
        @scroll.passive="handleChatScroll"
      >
      <div v-if="messages.length === 0" class="flex flex-col items-center justify-center h-full text-muted-foreground space-y-4">
        <div class="chat-empty-icon w-20 h-20 rounded-full flex items-center justify-center text-4xl animate-pulse">
          💕
        </div>
        <p class="text-lg">{{ t('chat.start') }}</p>
      </div>

      <template v-for="(msg, index) in messages" :key="msg.id">
      <div
        v-if="!cleanMode || !isCleanProcessMessage(msg) || (isCleanProcessAnchor(msg, index) && cleanProcessIsActive(index))"
        class="flex mb-4"
        :class="msg.role === 'user' ? 'justify-end' : 'justify-start'"
      >
        <template v-if="cleanMode && isCleanProcessAnchor(msg, index) && cleanProcessIsActive(index)">
          <div class="flex max-w-[85%] items-start space-x-2">
            <div class="chat-avatar w-9 h-9 rounded-md flex items-center justify-center text-xl flex-shrink-0">
              {{ getEmotionEmoji(msg.emotion) }}
            </div>
            <div class="flex flex-col min-w-0 max-w-full items-start">
              <div class="chat-bubble chat-bubble-assistant clean-thinking-bubble relative px-4 py-3 rounded-2xl text-sm leading-relaxed break-words">
                <div
                  class="clean-thinking-status"
                  role="status"
                  aria-live="polite"
                  :aria-busy="cleanProcessIsActive(index)"
                >
                  <div
                    class="streaming-dots clean-thinking-dots"
                    aria-label="Loading"
                  >
                    <i />
                    <i />
                    <i />
                  </div>
                  <span v-if="cleanProcessCurrentReasoning(index)" class="clean-thinking-reasoning">
                    {{ t('chat.deepThinking') }}：{{ cleanProcessCurrentReasoning(index) }}
                  </span>
                  <span v-else-if="cleanProcessCurrentToolName(index)" class="clean-thinking-tool-list">
                    {{ t('chat.cleanToolCall', { name: cleanProcessCurrentToolName(index) }) }}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </template>
        <template v-else>
        <div class="flex max-w-[85%] items-start space-x-2" :class="msg.role === 'user' ? 'flex-row-reverse space-x-reverse' : 'flex-row'">
          <!-- Avatar -->
          <template v-if="msg.role !== 'user' && msg.role !== 'tool'">
            <div class="chat-avatar w-9 h-9 rounded-md flex items-center justify-center text-xl flex-shrink-0">
              {{ getEmotionEmoji(msg.emotion) }}
            </div>
          </template>
          <template v-else-if="msg.role === 'tool' && (!cleanMode || hasInteractiveToolCard(msg))">
            <div class="w-9 h-9 rounded-md flex items-center justify-center flex-shrink-0 bg-secondary text-muted-foreground border border-border">
              <Wrench :size="16" />
            </div>
          </template>
          <div v-else-if="msg.role === 'user'" class="chat-avatar-me w-9 h-9 rounded-md flex items-center justify-center text-xs flex-shrink-0">
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
              <template v-if="msg.toolName === 'plan_create' && getCachedCard(msg.id, msg.content)">
                <div class="min-w-0">
                  <div v-if="!cleanMode" class="tool-call-caption">
                    {{ t('chat.toolCall', { name: toolDisplayName(msg) }) }}
                  </div>
                  <DecisionCard
                    :card="(getCachedCard(msg.id, msg.content) as unknown as UiCard)"
                    @action="onCardAction"
                  />
                </div>
              </template>
              <template v-else-if="msg.toolName === 'todo_write' && getCachedCard(msg.id, msg.content)">
                <div class="min-w-0">
                  <div v-if="!cleanMode" class="tool-call-caption">
                    {{ t('chat.toolCall', { name: toolDisplayName(msg) }) }}
                  </div>
                  <TodoCard
                    :card="(getCachedCard(msg.id, msg.content) as unknown as UiCard)"
                    @check="onCardCheck"
                  />
                </div>
              </template>
              <template v-else-if="msg.toolName === 'update_plan' && getCachedCard(msg.id, msg.content)">
                <div class="min-w-0">
                  <div v-if="!cleanMode" class="tool-call-caption">
                    {{ t('chat.toolCall', { name: toolDisplayName(msg) }) }}
                  </div>
                  <TodoCard
                    :card="(getCachedCard(msg.id, msg.content) as unknown as UiCard)"
                  />
                </div>
              </template>
              <!-- Default tool output rendering -->
              <div
                v-else
                class="tool-message rounded-lg border text-sm overflow-hidden bg-card"
                :class="{
                  'border-border': msg.toolStatus === 'running',
                  'border-success bg-success-soft': msg.toolStatus === 'success',
                  'border-destructive bg-destructive-soft': msg.toolStatus === 'error'
                }"
              >
                <!-- Tool Header -->
                <div class="px-3 py-2 flex items-center space-x-2">
                  <div v-if="msg.toolStatus === 'running'" class="text-muted-foreground" aria-label="Loading">
                    <div class="streaming-dots tool-streaming-dots">
                      <i />
                      <i />
                      <i />
                    </div>
                  </div>
                  <div v-else-if="msg.toolStatus === 'success'" class="text-success">
                    <CheckCircle2 :size="14" />
                  </div>
                  <div v-else class="text-destructive">
                    <XCircle :size="14" />
                  </div>

                  <span class="font-medium" :class="{
                    'text-muted-foreground': msg.toolStatus === 'running',
                    'text-success': msg.toolStatus === 'success',
                    'text-destructive': msg.toolStatus === 'error'
                  }">
                    {{ t('chat.toolCall', { name: toolDisplayName(msg) }) }}
                  </span>

                  <span v-if="msg.toolStatus !== 'running'" class="text-xs px-1.5 py-0.5 rounded bg-secondary text-muted-foreground border border-border">
                    {{ msg.toolStatus === 'success' ? t('chat.toolSuccess') : t('chat.toolFailed') }}
                  </span>
                </div>

                <!-- Tool Details Toggle -->
                <div
                  v-if="!cleanMode && msg.toolStatus !== 'running' && (msg.toolResult || toolResultRef(msg))"
                  class="px-3 pb-1 text-xs text-muted-foreground break-all whitespace-pre-wrap"
                >
                  {{ toolResultPreview(msg) }}
                </div>
                <div v-if="msg.toolStatus !== 'running'" class="px-3 pb-2 flex justify-end">
                  <button
                    @click="toggleTool(msg.id)"
                    class="ui-button ui-button--ghost ui-button--compact"
                  >
                    <span>{{ expandedTools[msg.id] ? t('chat.hideDetails') : t('chat.viewDetails') }}</span>
                    <component :is="expandedTools[msg.id] ? ChevronDown : ChevronRight" :size="12" />
                  </button>
                </div>

                <!-- Tool Details Content -->
                <div v-if="expandedTools[msg.id]" class="border-t border-border bg-muted p-3 text-xs space-y-2">
                  <div v-if="msg.toolCallId">
                    <div class="font-semibold text-muted-foreground mb-1">tool_call_id</div>
                    <div class="bg-card border border-border rounded p-2 font-mono text-muted-foreground break-all whitespace-pre-wrap">{{ msg.toolCallId }}</div>
                  </div>
                  <div>
                    <div class="font-semibold text-muted-foreground mb-1">{{ t('chat.inputArgs') }}</div>
                    <div class="bg-secondary rounded p-2 font-mono text-muted-foreground break-all whitespace-pre-wrap">{{ msg.toolArgs }}</div>
                  </div>
                  <div v-if="toolResultRef(msg)" class="space-y-2">
                    <div>
                      <div class="font-semibold text-muted-foreground mb-1">{{ t('chat.artifactSize') }}</div>
                      <div class="bg-card border border-border rounded p-2 font-mono text-muted-foreground break-all">
                        {{ toolResultRef(msg)?.char_count }} chars / {{ toolResultRef(msg)?.byte_count }} bytes
                      </div>
                    </div>
                    <div>
                      <div class="font-semibold text-muted-foreground mb-1">{{ t('chat.artifactId') }}</div>
                      <div class="bg-card border border-border rounded p-2 font-mono text-muted-foreground break-all">{{ toolResultRef(msg)?.artifact_id }}</div>
                    </div>
                    <div>
                      <div class="font-semibold text-muted-foreground mb-1">{{ t('chat.readHint') }}</div>
                      <div class="text-muted-foreground break-words">{{ toolResultRef(msg)?.read_hint }}</div>
                    </div>
                    <button type="button" class="inline-flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground" @click="copyArtifactReference(msg)">
                      <Copy :size="12" />
                      <span>{{ t('chat.copyArtifactReference') }}</span>
                    </button>
                  </div>
                  <div v-else-if="msg.toolResult">
                    <div class="font-semibold text-muted-foreground mb-1">{{ t('chat.execResult') }}</div>
                    <div class="bg-card border border-border rounded p-2 font-mono text-muted-foreground max-h-40 overflow-y-auto break-all whitespace-pre-wrap">{{ msg.toolResult }}</div>
                  </div>
                </div>

                <div v-if="hasRawMeta(msg)" class="border-t border-border bg-card px-3 py-2">
                  <button
                    @click="toggleRawMeta(msg.id)"
                    class="ui-button ui-button--ghost ui-button--compact"
                  >
                    <span>{{ expandedRawMeta[msg.id] ? t('chat.hideRawMeta') : t('chat.viewRawMeta') }}</span>
                    <component :is="expandedRawMeta[msg.id] ? ChevronDown : ChevronRight" :size="12" />
                  </button>
                  <div
                    v-if="expandedRawMeta[msg.id]"
                    class="mt-2 bg-muted border border-border rounded p-2 font-mono text-[11px] text-muted-foreground max-h-52 overflow-y-auto whitespace-pre-wrap break-all"
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
                { 'chat-bubble-has-reasoning': Boolean(msg.reasoning) && !cleanMode },
              ]"
            >
              <!-- Reasoning Block -->
              <div v-if="msg.reasoning && !cleanMode" class="streaming-reasoning-section">
                <ThinkingBlock
                  :content="msg.reasoning"
                  :thinking-ms="0"
                  :expanded="expandedReasoning[msg.id]"
                  @update:expanded="setReasoningExpanded(msg.id, $event)"
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

              <div v-if="hasRawMeta(msg) && !cleanMode" class="mb-2 rounded border border-border bg-card overflow-hidden">
                <div
                  @click="toggleRawMeta(msg.id)"
                  class="flex items-center justify-between px-2 py-1.5 cursor-pointer hover:bg-overlay transition-colors select-none"
                >
                  <span class="text-xs text-muted-foreground">{{ t('chat.rawMeta') }}</span>
                  <component :is="expandedRawMeta[msg.id] ? ChevronDown : ChevronRight" :size="14" class="text-muted-foreground" />
                </div>
                <div v-if="expandedRawMeta[msg.id]" class="px-3 py-2 border-t border-border bg-muted text-xs text-muted-foreground">
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

              <!-- Provider status badges on the streaming agent message -->
              <div
                v-if="msg.isStreaming && msg.retryStatus"
                class="mt-2 inline-flex items-center gap-1 rounded-md border border-warning bg-warning-soft px-2 py-0.5 text-xs text-warning"
              >
                {{ t('chat.retrying', { attempt: msg.retryStatus.attempt, max: msg.retryStatus.maxRetries }) }}
              </div>
              <div
                v-else-if="msg.isStreaming && msg.stalled"
                class="mt-2 inline-flex items-center gap-1 rounded-md border border-border bg-muted px-2 py-0.5 text-xs text-muted-foreground"
              >
                {{ t('chat.stalled') }}
              </div>
            </div>

            <!-- Message Actions: 时间戳 + 操作按钮 -->
            <div
              v-if="msg.role !== 'tool'"
              class="msg-actions"
              :class="msg.role === 'user' ? 'justify-end' : 'justify-start'"
            >
              <span class="text-[10px] text-muted-foreground">{{ formatTime(msg.timestamp) }}</span>
              <!-- 复制按钮（启用） -->
              <button
                class="ui-button ui-button--ghost msg-action-btn ui-button--compact ui-button--icon"
                @click="copyMessage(msg.content)"
                :title="t('chat.copy')"
              >
                <Copy :size="12" />
              </button>
              <!-- 编辑按钮（用户消息，disabled占位） -->
              <button
                v-if="msg.role === 'user'"
                class="ui-button ui-button--ghost msg-action-btn ui-button--compact ui-button--icon"
                disabled
                :title="t('chat.edit') + ' (' + t('chat.pending') + ')'"
              >
                <Edit :size="12" />
              </button>
              <!-- 重生成按钮（助手消息） -->
              <button
                v-if="msg.role === 'agent'"
                class="ui-button ui-button--ghost msg-action-btn ui-button--compact ui-button--icon"
                :disabled="isTyping"
                :title="t('chat.regenerate')"
                @click="emit('regenerate', msg.id)"
              >
                <RefreshCw :size="12" />
              </button>
              <!-- 朗读按钮（助手消息，DN-6C replay — 身份随 run 栅栏） -->
              <button
                v-if="msg.role === 'agent' && msg.content"
                class="ui-button ui-button--ghost msg-action-btn ui-button--compact ui-button--icon"
                :title="t('chat.voiceReplay')"
                @click="void voice.requestReply(msg.runId, msg.content)"
              >
                <Volume2 :size="12" />
              </button>
              <!-- 回退按钮（disabled占位） -->
              <button
                class="ui-button ui-button--ghost msg-action-btn ui-button--compact ui-button--icon"
                disabled
                :title="t('chat.rewind') + ' (' + t('chat.pending') + ')'"
              >
                <Rewind :size="12" />
              </button>
              <!-- 分叉按钮（disabled占位） -->
              <button
                class="ui-button ui-button--ghost msg-action-btn ui-button--compact ui-button--icon"
                disabled
                :title="t('chat.fork') + ' (' + t('chat.pending') + ')'"
              >
                <GitFork :size="12" />
              </button>
            </div>

            <!-- Tool消息时间戳 -->
            <span
              v-else
              class="text-[10px] text-muted-foreground mt-1 text-left"
            >
              {{ formatTime(msg.timestamp) }}
            </span>
          </div>
        </div>
        </template>
      </div>
      </template>



      <PlanApprovalCard
        v-if="approvalPlan"
        :plan="approvalPlan"
        :approving="approvingPlan"
        @approve="handleApprovePlan"
        @start-goal="emit('start-goal', $event)"
        @revoke="emit('revoke-plan', $event)"
        @refresh="emit('refresh-plan')"
      />

      <AskUserQuestionCard
        v-for="question in askUserQuestions"
        :key="question.question_id"
        :question="question"
        @answer="emit('answer-ask-user', $event)"
        @cancel="emit('cancel-ask-user', $event)"
      />

      <!-- Typing Indicator -->
      <!-- Removed separate Typing Indicator as it is now integrated into the message bubble -->

    </div>

    <div
      v-if="activePlanRuntime && !approvalPlan"
      class="active-plan-todo-panel"
    >
      <div v-if="planExecutionError" class="active-plan-execution-error" role="alert">
        <span>{{ planExecutionError }}</span>
        <button
          type="button"
          :disabled="approvingPlan || isTyping"
          @click.stop="emit('resume-plan')"
        >
          {{ approvingPlan ? '正在重试...' : '继续执行' }}
        </button>
      </div>
      <div class="active-plan-todo-bar" role="button" tabindex="0" @click="openPlanTasks" @keydown.enter="openPlanTasks">
        <ClipboardList :size="16" class="active-plan-todo-icon" />
        <div class="active-plan-todo-content">
          <span class="active-plan-todo-plan">{{ activePlanRuntime.title }}</span>
          <span v-if="activePlanTodo" class="active-plan-todo-title">{{ activePlanTodo.title }}</span>
          <span v-else class="active-plan-todo-title">{{ activePlanRuntime.todos.length === 0 ? '无执行清单，按批准计划执行' : activePlanRuntime.phase }}</span>
        </div>
        <span class="active-plan-todo-progress">{{ planProgressText }}</span>
        <Loader2 v-if="activePlanTodo?.status === 'InProgress'" :size="15" class="text-warning animate-spin" />
        <button
          type="button"
          class="ui-button ui-button--ghost ui-button--compact active-plan-todo-toggle"
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
          <CheckCircle2 v-if="todo.status === 'Completed'" :size="14" class="text-success" />
          <Loader2 v-else-if="todo.status === 'InProgress'" :size="14" class="text-warning animate-spin" />
          <Clock v-else :size="14" class="text-muted-foreground" />
          <span>{{ todo.title }}</span>
          <ChevronRight :size="14" class="active-plan-task-chevron" />
        </button>
      </div>
      <div v-if="activePlanTodoExpanded" class="active-plan-todo-details">
        <div class="active-plan-todo-details-title">{{ activePlanRuntime.title }}</div>
        <div v-if="activePlanRuntime.steps.length > 0" class="active-plan-todo-details-list">
          <div v-for="step in activePlanRuntime.steps" :key="step.id" class="active-plan-todo-detail-item">
            <CheckCircle2 v-if="step.status === 'Completed'" :size="13" class="text-success" />
            <Loader2 v-else-if="step.status === 'InProgress'" :size="13" class="text-warning animate-spin" />
            <Clock v-else :size="13" class="text-muted-foreground" />
            <span>{{ step.ordinal + 1 }}. {{ step.title }}</span>
          </div>
        </div>
        <div v-else class="active-plan-todo-details-list">
          <div v-for="todo in activePlanRuntime.todos" :key="todo.id" class="active-plan-todo-detail-item">
            <CheckCircle2 v-if="todo.status === 'Completed'" :size="13" class="text-success" />
            <Loader2 v-else-if="todo.status === 'InProgress'" :size="13" class="text-warning animate-spin" />
            <Clock v-else :size="13" class="text-muted-foreground" />
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
          <button type="button" class="ui-button ui-button--ghost ui-button--compact todo-status-close" title="关闭" aria-label="关闭" @click="closeTodoStatus">
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
        <!-- 顶部工具栏 -->
        <div class="chat-input-toolbar">
          <!-- 执行模式选择 -->
          <div class="relative">
            <button
              @click="showModeMenu = !showModeMenu"
              class="ui-button ui-button--ghost ui-button--compact toolbar-btn mode-selector"
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

          <!-- 语音按钮 -->
          <!-- 思考模式选择 -->
          <ThinkingToggle v-model="thinkingMode" />

          <!-- 权限模式选择 -->
          <div class="relative">
            <button
              @click="showPermissionMenu = !showPermissionMenu"
              class="ui-button ui-button--ghost ui-button--compact toolbar-btn permission-btn"
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

          <!-- 历史记录 + 审批中心（工具栏右侧） -->
          <span class="toolbar-divider" />
          <div class="chat-corner-actions">
            <button
              type="button"
              class="ui-button ui-button--ghost ui-button--compact toolbar-btn"
              :title="convSidebarOpen ? t('convSidebar.close') : t('convSidebar.open')"
              :aria-label="convSidebarOpen ? t('convSidebar.close') : t('convSidebar.open')"
              :aria-expanded="convSidebarOpen"
              @click="convSidebarOpen = !convSidebarOpen"
            >
              <Clock :size="14" />
            </button>
            <button
              type="button"
              class="ui-button ui-button--ghost ui-button--compact toolbar-btn approval-center-icon-btn"
              :class="{ active: approvalCenterOpen }"
              :title="t('approvalCenter.open', { count: approvalPendingCount || 0 })"
              :aria-label="t('approvalCenter.open', { count: approvalPendingCount || 0 })"
              :aria-expanded="!!approvalCenterOpen"
              @click="emit('update:approval-center-open', !approvalCenterOpen)"
            >
              <ShieldCheck :size="14" />
              <strong
                v-if="approvalPendingCount"
                class="approval-pending-badge"
                aria-live="polite"
              >{{ approvalPendingCount }}</strong>
            </button>
          </div>
        </div>

        <!-- 已选图片附件 -->
        <div v-if="pendingImages.length" class="image-attachments">
          <div v-for="(att, index) in pendingImages" :key="index" class="image-chip">
            <img :src="`data:${att.mime_type};base64,${att.data}`" class="image-thumb" :alt="att.name || 'image'" />
            <span class="image-name">{{ att.name || 'image' }}</span>
            <button class="ui-button ui-button--ghost ui-button--compact image-remove" @click="removeImage(index)" :title="t('chat.removeAttachment')">
              <X :size="12" />
            </button>
          </div>
        </div>
        <div v-if="attachmentError" class="attachment-error">{{ attachmentError }}</div>
        <PersonaSetupGate v-if="personaSetupRequired" />
        <input
          ref="fileInputRef"
          type="file"
          accept="image/png,image/jpeg,image/gif,image/webp"
          multiple
          hidden
          @change="handlePickImages"
        />

        <!-- 主输入区 -->
        <div class="chat-input-main">
          <textarea
            ref="inputRef"
            v-model="input"
            @input="adjustInputHeight"
            @keydown="handleKeyDown"
            :placeholder="getPlaceholder"
            class="ui-input ui-input--embedded chat-textarea"
            rows="1"
          />
        </div>

        <!-- 底部操作栏 -->
        <div class="chat-input-footer">
          <!-- 上下文使用指示器 -->
          <div class="context-usage" :title="contextUsageTitle">
            <svg class="context-ring" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="9" fill="none" stroke="var(--border)" stroke-width="2"/>
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
              class="ui-button ui-button--ghost input-action-btn left ui-button--compact ui-button--icon"
              :title="t('chat.newSession')"
            >
              <Plus :size="18" />
            </button>

            <button
              class="ui-button ui-button--ghost input-action-btn ui-button--compact ui-button--icon"
              :class="{ recording: isRecording, speaking: isSpeaking }"
              :title="micTitle"
              @click="handleMicClick"
            >
              <Loader2 v-if="isTranscribing" :size="18" class="spin" />
              <Square v-else-if="isSpeaking" :size="18" />
              <Mic v-else :size="18" />
            </button>

            <!-- 图片附件按钮 -->
            <button
              class="ui-button ui-button--ghost input-action-btn ui-button--compact ui-button--icon"
              :title="t('chat.attachImage')"
              @click="triggerImagePicker"
            >
              <ImagePlus :size="18" />
            </button>

            <!-- 发送/停止按钮 -->
            <button
              v-if="isTyping"
              @click="handleStop"
              class="ui-button ui-button--ghost input-action-btn send stop-btn ui-button--compact ui-button--icon"
              :title="t('chat.stop')"
            >
              <Square :size="18" />
            </button>
            <button
              v-else
              @click="handleSend"
              :disabled="!input.trim() || personaSetupRequired"
              class="ui-button ui-button--ghost input-action-btn send ui-button--compact ui-button--icon"
              :class="{ disabled: !input.trim() || personaSetupRequired }"
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
      @select="handleSelectSession"
      @delete="(key) => emit('delete-session', key)"
      @new="handleNewSession"
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
/* Chat corner actions: history + approval center (right side of input toolbar) */
.chat-corner-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 2px;
}

.approval-center-icon-btn {
  position: relative;
}

.approval-center-icon-btn.active {
  color: var(--warning);
  border-color: var(--warning, var(--warning));
  background: var(--warning-soft);
}

.approval-pending-badge {
  position: absolute;
  top: -6px;
  right: -6px;
  display: grid;
  min-width: 18px;
  height: 18px;
  place-items: center;
  border-radius: 999px;
  padding: 0 5px;
  background: var(--warning-soft);
  color: var(--warning);
  font-size: 10px;
  font-weight: 700;
  line-height: 1;
  box-shadow: var(--shadow-sm);
}

.active-plan-todo-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  min-width: 0;
  margin: 0 16px 8px;
  padding: 8px 10px;
  border: 1px solid var(--warning);
  border-radius: 10px;
  background: var(--warning-soft);
  box-shadow: var(--shadow-sm);
}
.active-plan-todo-panel { flex-shrink: 0; min-width: 0; margin: 0 16px 8px; }
.active-plan-todo-panel .active-plan-todo-bar { margin: 0; }
.active-plan-execution-error { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-bottom: 8px; padding: 8px 10px; border: 1px solid var(--destructive); border-radius: 10px; color: var(--destructive); background: var(--destructive-soft); font-size: 12px; line-height: 1.4; }
.active-plan-execution-error span { min-width: 0; overflow-wrap: anywhere; }
.active-plan-execution-error button { flex: 0 0 auto; border: 1px solid var(--destructive); border-radius: 7px; padding: 5px 9px; color: var(--destructive); background: var(--destructive-soft); font-size: 11px; font-weight: 700; cursor: pointer; }
.active-plan-execution-error button:disabled { cursor: not-allowed; opacity: .6; }
.active-plan-todo-icon { flex: 0 0 auto; color: var(--warning); }
.active-plan-todo-content { display: flex; min-width: 0; flex: 1; align-items: baseline; gap: 8px; }
.active-plan-todo-plan { flex: 0 0 auto; max-width: 30%; overflow: hidden; color: var(--warning); font-size: 11px; font-weight: 700; text-overflow: ellipsis; white-space: nowrap; }
.active-plan-todo-title { min-width: 0; overflow: hidden; color: var(--warning); font-size: 12px; text-overflow: ellipsis; white-space: nowrap; }
.active-plan-todo-progress { flex: 0 0 auto; color: var(--warning); font-size: 11px; font-variant-numeric: tabular-nums; }
.active-plan-todo-toggle { display: flex; align-items: center; justify-content: center; flex: 0 0 auto; }
.active-plan-todo-details { padding: 9px 12px 10px; border: 1px solid var(--warning); border-top: 0; border-radius: 0 0 10px 10px; background: var(--warning-soft); }
.active-plan-todo-details-title { margin-bottom: 6px; color: var(--warning); font-size: 11px; font-weight: 700; }
.active-plan-todo-details-list { display: grid; gap: 5px; }
.active-plan-todo-detail-item { display: flex; align-items: center; gap: 6px; min-width: 0; color: var(--warning); font-size: 11px; }
.active-plan-todo-detail-item span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.active-plan-task-list { display: grid; gap: 4px; margin-top: -1px; padding: 8px; border: 1px solid var(--warning); border-top: 0; border-radius: 0 0 10px 10px; background: var(--card); }
.active-plan-task-list-title { padding: 2px 4px 5px; color: var(--warning); font-size: 11px; font-weight: 700; }
.active-plan-task-item { display: flex; align-items: center; gap: 7px; min-width: 0; padding: 7px 6px; border: 0; border-radius: 7px; color: var(--muted-foreground); background: transparent; font-size: 11px; text-align: left; cursor: pointer; }
.active-plan-task-item:hover { background: var(--warning-soft); }
.active-plan-task-item span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.active-plan-task-chevron { margin-left: auto; flex: 0 0 auto; color: var(--warning); }
.todo-status-overlay { position: absolute; inset: 0; z-index: 80; display: flex; align-items: center; justify-content: center; padding: 24px; background: var(--overlay); }
.todo-status-dialog { width: min(520px, 100%); max-height: min(80vh, 620px); overflow: auto; border: 1px solid var(--border); border-radius: 16px; background: var(--card); box-shadow: var(--shadow-lg); }
.todo-status-header { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; padding: 18px 20px; border-bottom: 1px solid var(--border); }
.todo-status-eyebrow { margin-bottom: 5px; color: var(--muted-foreground); font-size: 11px; }
.todo-status-header h2 { margin: 0; color: var(--foreground); font-size: 17px; font-weight: 700; }
.todo-status-close { display: flex; align-items: center; justify-content: center; }
.todo-status-body { display: grid; gap: 14px; padding: 18px 20px 22px; color: var(--foreground); }
.todo-status-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding-bottom: 10px; border-bottom: 1px solid var(--border); font-size: 13px; }
.todo-status-row span, .todo-status-section > span { color: var(--muted-foreground); font-size: 12px; }
.todo-status-row strong { font-size: 13px; }
.todo-status-section p { margin: 5px 0 0; color: var(--foreground); font-size: 13px; line-height: 1.6; white-space: pre-wrap; }
.todo-status-blocked { padding: 10px 12px; border-radius: 9px; background: var(--warning-soft); }

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
  box-shadow: var(--shadow-sm);
}

.conv-sidebar-scrim {
  position: fixed;
  inset: 0;
  z-index: 150;
  background: var(--overlay);
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

.tool-call-caption {
  margin-bottom: 6px;
  color: var(--muted-foreground);
  font-size: 12px;
  line-height: 1.4;
}

.clean-thinking-status {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  max-width: 100%;
  white-space: nowrap;
}

.clean-thinking-dots { flex: 0 0 auto; }
.clean-thinking-reasoning {
  min-width: 0;
  max-width: 100%;
  overflow: hidden;
  color: var(--muted-foreground);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.clean-thinking-tool-list {
  min-width: 0;
  overflow: hidden;
  color: var(--muted-foreground);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.streaming-reasoning-status {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 4px;
  color: var(--muted-foreground);
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
  background: var(--muted-foreground);
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
  background-color: var(--code-background);
  border-radius: 0.375rem;
  padding: 0.75rem;
  margin: 0.5rem 0;
  overflow-x: auto;
}

:deep(.markdown-body code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.85em;
  background-color: var(--accent);
  padding: 0.2em 0.4em;
  border-radius: 0.25rem;
}

:deep(.markdown-body pre code) {
  background-color: transparent;
  padding: 0;
  color: var(--code-foreground);
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
  border-left: 3px solid var(--border);
  padding-left: 0.75rem;
  color: var(--muted-foreground);
  margin: 0.5rem 0;
}

:deep(.markdown-body a) {
  color: var(--info, var(--info));
  text-decoration: underline;
}

:deep(.markdown-body a:hover) {
  color: var(--primary-hover);
}
.autodream-trigger-notice {
  min-width: min(360px, 100%);
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--primary) 28%, var(--border, var(--border)));
  border-radius: 10px;
  background: color-mix(in srgb, var(--primary) 7%, var(--card));
  color: var(--foreground);
}

.autodream-trigger-notice p {
  margin: 0;
  line-height: 1.45;
}

.autodream-trigger-notice__error {
  margin-top: 4px !important;
  color: var(--destructive);
  font-size: 12px;
}

.autodream-trigger-notice__action {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-top: 8px;
  padding: 0;
  border: 0;
  color: var(--primary);
  background: transparent;
  cursor: pointer;
  font-size: 12px;
  font-weight: 650;
}

.autodream-trigger-notice__action:hover {
  text-decoration: underline;
}

/* DN-2A image attachment chips */
.image-attachments {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 6px 4px;
}
.image-chip {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 6px;
  border: 1px solid var(--border, var(--border));
  border-radius: 8px;
  background: var(--card);
  font-size: 12px;
}
.image-thumb {
  width: 32px;
  height: 32px;
  object-fit: cover;
  border-radius: 5px;
}
.image-name {
  max-width: 140px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--foreground);
}
.image-remove {
  display: inline-flex;
  align-items: center;
}
.attachment-error {
  padding: 4px 6px;
  color: var(--destructive);
  font-size: 12px;
}
</style>
