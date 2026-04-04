<script setup lang="ts">
import { ref, computed, nextTick, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { Send, Square, Plus, Wrench, ChevronDown, ChevronRight, CheckCircle2, XCircle, Loader2, Brain, Paperclip, X } from 'lucide-vue-next';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css'; // 使用 GitHub Dark 风格
import { useI18n } from 'vue-i18n';
import { uploadFile, type FileAttachmentDto } from '../api/desktop';

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

const props = defineProps<{
  messages: Message[];
  isTyping: boolean;
  themeMode?: string;
  historyPrefs?: HistoryPrefs;
}>();

const emit = defineEmits<{
  (e: 'send', content: string, attachments?: FileAttachmentDto[]): void;
  (e: 'clear'): void;
  (e: 'stop'): void;
}>();

const input = ref('');
const messagesEndRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLTextAreaElement | null>(null);
const fileInputRef = ref<HTMLInputElement | null>(null);
const attachments = ref<FileAttachmentDto[]>([]);
const uploading = ref(false);

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

const handleFileSelect = async (event: Event) => {
  const target = event.target as HTMLInputElement;
  const files = target.files;
  if (!files || files.length === 0) return;

  uploading.value = true;
  try {
    for (const file of files) {
      const bytes = await file.arrayBuffer();
      const byteArray = Array.from(new Uint8Array(bytes));
      const attachment = await uploadFile(file.name, byteArray, 'gui');
      attachments.value.push(attachment);
    }
  } catch (error) {
    console.error('Failed to upload file:', error);
  } finally {
    uploading.value = false;
    if (fileInputRef.value) {
      fileInputRef.value.value = '';
    }
  }
};

const handleRemoveAttachment = (index: number) => {
  attachments.value.splice(index, 1);
};

const handleSend = () => {
  if (props.isTyping) return;
  const hasContent = input.value.trim() || attachments.value.length > 0;
  if (!hasContent) return;

  const currentAttachments = [...attachments.value];
  const message = input.value.trim() || (attachments.value.length > 0 ? '[文件]' : '');
  emit('send', message, currentAttachments);
  input.value = '';
  attachments.value = [];
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
</script>

<template>
  <div class="chat-shell flex flex-col h-full relative overflow-hidden" :class="`theme-${themeMode || 'love'}`">
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
        <div class="flex max-w-[80%] items-start space-x-2" :class="msg.role === 'user' ? 'flex-row-reverse space-x-reverse' : 'flex-row'">
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
              class="chat-bubble relative px-3 py-2 rounded-md text-sm leading-relaxed shadow-sm break-words"
              :class="msg.role === 'user' ? 'chat-bubble-user' : 'chat-bubble-assistant'"
            >
              <!-- Tail -->
              <div
                class="absolute top-3 w-0 h-0 border-solid border-4"
                :class="msg.role === 'user' ? 'chat-bubble-tail-user' : 'chat-bubble-tail-assistant'"
              />

              <!-- Reasoning Block -->
              <div v-if="msg.reasoning" class="mb-2 rounded border border-gray-200/50 bg-white/40 overflow-hidden">
                 <!-- Header -->
                 <div 
                    @click="toggleReasoning(index)"
                    class="flex items-center justify-between px-2 py-1.5 cursor-pointer hover:bg-black/5 transition-colors select-none"
                 >
                    <div class="flex items-center space-x-2 text-xs">
                       <Brain :size="14" :class="msg.isThinking ? 'animate-pulse text-pink-500' : 'text-gray-400'" />
                       <span :class="msg.isThinking ? 'text-pink-600 font-medium' : 'text-gray-500'">
                          {{ msg.isThinking ? t('chat.thinking') : t('chat.thoughtProcess') }}
                       </span>
                    </div>
                    <component :is="expandedReasoning[index] ? ChevronDown : ChevronRight" :size="14" class="text-gray-400" />
                 </div>
                 
                 <!-- Content -->
                 <div v-if="expandedReasoning[index]" class="px-3 py-2 border-t border-gray-100/50 bg-gray-50/30 text-xs text-gray-600">
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
              <div v-else class="markdown-body" v-html="md.render(msg.content)"></div>
            </div>
            
            <!-- Timestamp -->
            <span
              class="text-[10px] text-gray-400 mt-1"
              :class="msg.role === 'user' ? 'text-right' : 'text-left'"
            >
              {{ msg.timestamp ? new Date(msg.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : '' }}
            </span>
          </div>
        </div>
      </div>

      <!-- Typing Indicator -->
      <!-- Removed separate Typing Indicator as it is now integrated into the message bubble -->
      
      <div ref="messagesEndRef" />
    </div>

    <!-- Input Area -->
    <div class="chat-input-bar border-t p-4 z-20">
      <input
        type="file"
        ref="fileInputRef"
        @change="handleFileSelect"
        class="hidden"
        multiple
      />
      <!-- Attachment Preview -->
      <div v-if="attachments.length > 0" class="mb-2 flex flex-wrap gap-2">
        <div
          v-for="(attachment, index) in attachments"
          :key="attachment.file_id"
          class="flex items-center gap-1 bg-gray-100 rounded-lg px-2 py-1 text-xs"
        >
          <Paperclip :size="12" class="text-gray-500" />
          <span class="text-gray-700 truncate max-w-[100px]">{{ attachment.filename }}</span>
          <button
            @click="handleRemoveAttachment(index)"
            class="text-gray-400 hover:text-red-500"
          >
            <X :size="12" />
          </button>
        </div>
      </div>
      <div class="flex items-center space-x-3 bg-white rounded-xl border border-gray-200 px-2 py-2 shadow-sm focus-within:ring-2 focus-within:ring-pink-500/20 focus-within:border-pink-500 transition-all">
        <button
          @click="handleClear"
          class="p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-lg transition-all flex-shrink-0"
          :title="t('chat.newSession')"
        >
          <Plus :size="20" />
        </button>
        <button
          @click="fileInputRef?.click()"
          :disabled="uploading"
          class="p-2 text-gray-400 hover:text-pink-500 hover:bg-pink-50 rounded-lg transition-all flex-shrink-0"
          :title="uploading ? t('chat.uploading') : t('chat.attachFile')"
        >
          <Loader2 v-if="uploading" :size="20" class="animate-spin" />
          <Paperclip v-else :size="20" />
        </button>

        <div class="flex-1 relative flex items-center">
          <textarea
            ref="inputRef"
            v-model="input"
            @keydown="handleKeyDown"
            :placeholder="t('chat.placeholder')"
            class="w-full bg-transparent text-sm resize-none focus:outline-none placeholder-gray-400 py-2 max-h-[120px]"
            rows="1"
            style="min-height: 24px;"
          />
        </div>
        
        <button
          @click="handleStop"
          :disabled="!isTyping"
          class="p-2 rounded-lg transition-all flex items-center justify-center flex-shrink-0"
          :class="!isTyping ? 'bg-gray-100 text-gray-400 cursor-not-allowed' : 'bg-red-500 text-white shadow-md shadow-red-500/20 hover:bg-red-600 hover:shadow-lg hover:scale-105 active:scale-95'"
          :title="t('chat.stopGeneration')"
        >
          <Square :size="18" />
        </button>
        <button
          @click="handleSend"
          :disabled="(!input.trim() && !attachments.length) || isTyping"
          class="p-2 rounded-lg transition-all flex items-center justify-center flex-shrink-0"
          :class="(!input.trim() && !attachments.length) || isTyping ? 'bg-gray-100 text-gray-400 cursor-not-allowed' : 'bg-gradient-to-r from-pink-500 to-pink-600 text-white shadow-md shadow-pink-500/20 hover:shadow-lg hover:scale-105 active:scale-95'"
        >
          <Send :size="18" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
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
