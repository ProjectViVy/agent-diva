<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Send, Square, Plus, Wrench, ChevronDown, ChevronRight, CheckCircle, CheckCircle2, XCircle, Loader2, Brain, Copy, Edit, RefreshCw, Rewind, GitFork, Paperclip, Mic, Settings2, Zap, Clock, Shield, Sparkles } from 'lucide-vue-next';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css'; // 使用 GitHub Dark 风格
import { useI18n } from 'vue-i18n';
import ConversationSidebar from './ConversationSidebar.vue';

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
}

const expandedTools = ref<Record<number, boolean>>({});
const expandedReasoning = ref<Record<number, boolean>>({});
const expandedRawMeta = ref<Record<number, boolean>>({});

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

const toggleTool = (index: number) => {
  expandedTools.value[index] = !expandedTools.value[index];
};

const toggleReasoning = (index: number) => {
  expandedReasoning.value[index] = !expandedReasoning.value[index];
};

const toggleRawMeta = (index: number) => {
  expandedRawMeta.value[index] = !expandedRawMeta.value[index];
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
}>();

const emit = defineEmits<{
  (e: 'send', content: string): void;
  (e: 'clear'): void;
  (e: 'stop'): void;
  (e: 'select-session', sessionKey: string): void;
  (e: 'delete-session', sessionKey: string): void;
  (e: 'new-session'): void;
  (e: 'toggle-pin', sessionKey: string): void;
  (e: 'rename-session', sessionKey: string, title: string): void;
}>();

const input = ref('');
const messagesEndRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLTextAreaElement | null>(null);
const inputHeight = ref(24); // 动态输入框高度

// 右侧会话侧边栏状态
const convSidebarOpen = ref(false);
const convSidebarRef = ref<InstanceType<typeof ConversationSidebar> | null>(null);

// 输入区域状态
const showModeMenu = ref(false);
const showPermissionMenu = ref(false);
const execMode = ref<'agent' | 'plan' | 'ask'>('agent');
const permissionMode = ref<'cautious' | 'smart' | 'trusted'>('smart');
// const showAttachments = ref(false); // 预留
const isRecording = ref(false);
// const recordingDuration = ref(0); // 预留
const showDeepThinking = ref(false);
// const thinkingDepth = ref<'low' | 'medium' | 'high'>('medium'); // 预留

const effectiveHistoryPrefs = computed<HistoryPrefs>(() => ({
  ...defaultHistoryPrefs,
  ...(props.historyPrefs ?? {}),
}));

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
  }

  // Auto-expand structured sections based on local preferences
  newMessages.forEach((msg, index) => {
    if (expandedReasoning.value[index] === undefined && msg.reasoning) {
      expandedReasoning.value[index] = effectiveHistoryPrefs.value.autoExpandReasoning || !!msg.isThinking;
    }
    if (expandedTools.value[index] === undefined && msg.role === 'tool') {
      expandedTools.value[index] = effectiveHistoryPrefs.value.autoExpandToolDetails;
    }
    if (expandedRawMeta.value[index] === undefined && hasRawMeta(msg)) {
      expandedRawMeta.value[index] = effectiveHistoryPrefs.value.showRawMetaByDefault;
    }
  });
  scrollToBottom();
}, { deep: true });

onMounted(() => {
  scrollToBottom();
  inputRef.value?.focus();
});

const handleSend = () => {
  if (!input.value.trim() || props.isTyping) return;
  emit('send', input.value);
  input.value = '';
  nextTick(() => {
    adjustInputHeight();
  });
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
        v-for="(msg, index) in messages"
        :key="index"
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
          <div class="flex flex-col min-w-0 max-w-full">
            <!-- Tool Message -->
            <div 
              v-if="msg.role === 'tool'"
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
                  @click="toggleTool(index)"
                  class="text-[10px] flex items-center space-x-1 text-gray-400 hover:text-gray-600 transition-colors"
                >
                  <span>{{ expandedTools[index] ? t('chat.hideDetails') : t('chat.viewDetails') }}</span>
                  <component :is="expandedTools[index] ? ChevronDown : ChevronRight" :size="12" />
                </button>
              </div>

              <!-- Tool Details Content -->
              <div v-if="expandedTools[index]" class="border-t border-gray-100 bg-gray-50/50 p-3 text-xs space-y-2">
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
                  @click="toggleRawMeta(index)"
                  class="text-[10px] flex items-center space-x-1 text-gray-400 hover:text-gray-600 transition-colors"
                >
                  <span>{{ expandedRawMeta[index] ? t('chat.hideRawMeta') : t('chat.viewRawMeta') }}</span>
                  <component :is="expandedRawMeta[index] ? ChevronDown : ChevronRight" :size="12" />
                </button>
                <div
                  v-if="expandedRawMeta[index]"
                  class="mt-2 bg-gray-50 border border-gray-200 rounded p-2 font-mono text-[11px] text-gray-600 max-h-52 overflow-y-auto whitespace-pre-wrap break-all"
                >
                  {{ renderRawMeta(msg) }}
                </div>
              </div>
            </div>

            <!-- Normal Message -->
            <div
              v-else
              class="chat-bubble relative px-4 py-3 rounded-2xl text-sm leading-relaxed break-words"
              :class="msg.role === 'user' ? 'chat-bubble-user' : 'chat-bubble-assistant'"
            >
              <!-- Reasoning Block - OpenAkita 折叠卡片样式 -->
              <div v-if="msg.reasoning" class="reasoning-card mb-3">
                 <!-- Header -->
                 <div 
                    @click="toggleReasoning(index)"
                    class="reasoning-header"
                 >
                    <div class="reasoning-header-left">
                       <ChevronRight :size="12" class="reasoning-chevron" :class="{ expanded: expandedReasoning[index] }" />
                       <Brain :size="14" :class="msg.isThinking ? 'thinking-active' : 'thinking-inactive'" />
                       <span class="reasoning-label">
                          <template v-if="msg.isThinking">
                             <Loader2 :size="12" class="animate-spin" />
                             {{ t('chat.reasoningProcessing') }}
                          </template>
                          <template v-else>
                             {{ t('chat.reasoningThought', { time: 'X.X' }) }}
                          </template>
                       </span>
                    </div>
                 </div>
                 
                 <!-- Content -->
                 <div v-if="expandedReasoning[index]" class="reasoning-content">
                    <div class="markdown-body" v-html="md.render(msg.reasoning)"></div>
                 </div>
              </div>

              <div v-if="hasRawMeta(msg)" class="mb-2 rounded border border-gray-200/50 bg-white/40 overflow-hidden">
                <div
                  @click="toggleRawMeta(index)"
                  class="flex items-center justify-between px-2 py-1.5 cursor-pointer hover:bg-black/5 transition-colors select-none"
                >
                  <span class="text-xs text-gray-500">{{ t('chat.rawMeta') }}</span>
                  <component :is="expandedRawMeta[index] ? ChevronDown : ChevronRight" :size="14" class="text-gray-400" />
                </div>
                <div v-if="expandedRawMeta[index]" class="px-3 py-2 border-t border-gray-100/50 bg-gray-50/30 text-xs text-gray-600">
                  <div class="font-mono whitespace-pre-wrap break-all">{{ renderRawMeta(msg) }}</div>
                </div>
              </div>
              
              <!-- Content or Loading -->
              <div v-if="!msg.content && msg.role === 'agent' && msg.isStreaming" class="flex space-x-1 py-1">
                 <div class="w-1.5 h-1.5 bg-gray-400 rounded-full animate-bounce" style="animation-delay: 0s" />
                 <div class="w-1.5 h-1.5 bg-gray-400 rounded-full animate-bounce" style="animation-delay: 0.1s" />
                 <div class="w-1.5 h-1.5 bg-gray-400 rounded-full animate-bounce" style="animation-delay: 0.2s" />
              </div>
              <div v-else>
                <div class="markdown-body" v-html="md.render(msg.content)"></div>
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
              <!-- 重生成按钮（助手消息，disabled占位） -->
              <button
                v-if="msg.role === 'agent'"
                class="msg-action-btn"
                disabled
                :title="t('chat.regenerate') + ' (' + t('chat.pending') + ')'"
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

      <!-- Typing Indicator -->
      <!-- Removed separate Typing Indicator as it is now integrated into the message bubble -->
      
      <div ref="messagesEndRef" />
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

          <div class="toolbar-divider"></div>

          <!-- 附件按钮 -->
          <button class="toolbar-btn" :title="t('chat.attachment')">
            <Paperclip :size="14" />
          </button>

          <!-- 语音按钮 -->
          <button 
            class="toolbar-btn" 
            :class="{ recording: isRecording }"
            :title="t('chat.voice')"
            @click="isRecording = !isRecording"
          >
            <Mic :size="14" />
          </button>

          <!-- 深度思考按钮 -->
          <button 
            class="toolbar-btn"
            :class="{ active: showDeepThinking }"
            :title="t('chat.deepThinking')"
            @click="showDeepThinking = !showDeepThinking"
          >
            <Brain :size="14" />
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
            :placeholder="getPlaceholder"
            class="chat-textarea"
            rows="1"
          />
        </div>

        <!-- 底部操作栏 -->
        <div class="chat-input-footer">
          <!-- 上下文使用指示器 -->
          <div class="context-usage">
            <svg class="context-ring" viewBox="0 0 24 24">
              <circle cx="12" cy="12" r="9" fill="none" stroke="#e5e7eb" stroke-width="2"/>
              <circle cx="12" cy="12" r="9" fill="none" stroke="var(--brand, #ec4899)" stroke-width="2" stroke-dasharray="56.5" stroke-dashoffset="28.25" stroke-linecap="round" transform="rotate(-90 12 12)"/>
            </svg>
            <span class="context-text">50%</span>
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
              :disabled="!input.trim()"
              class="input-action-btn send"
              :class="{ disabled: !input.trim() }"
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
      @close="convSidebarOpen = false"
      class="conv-sidebar-wrapper"
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

/* Conversation Sidebar Wrapper */
.conv-sidebar-wrapper {
  flex-shrink: 0;
  height: 100%;
}

/* Scoped styles if needed, but we rely on global tailwind classes mostly */
:deep(.markdown-body) {
  font-size: 0.875rem;
  line-height: 1.6;
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
