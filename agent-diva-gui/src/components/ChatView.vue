<script setup lang="ts">
import { ref, nextTick, watch, onMounted } from 'vue';
import { Send, Trash2, Wrench, ChevronDown, ChevronRight, CheckCircle2, XCircle, Loader2, Brain } from 'lucide-vue-next';
import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';
import 'highlight.js/styles/github-dark.css'; // 使用 GitHub Dark 风格

const md = new MarkdownIt({
  html: false, // 禁用 HTML 标签以防止 XSS
  linkify: true,
  breaks: true,
  highlight: function (str, lang) {
    if (lang && hljs.getLanguage(lang)) {
      try {
        return '<pre class="hljs"><code>' +
               hljs.highlight(str, { language: lang, ignoreIllegals: true }).value +
               '</code></pre>';
      } catch (__) {}
    }

    return '<pre class="hljs"><code>' + md.utils.escapeHtml(str) + '</code></pre>';
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
}

const expandedTools = ref<Record<number, boolean>>({});
const expandedReasoning = ref<Record<number, boolean>>({});

const toggleTool = (index: number) => {
  expandedTools.value[index] = !expandedTools.value[index];
};

const toggleReasoning = (index: number) => {
  expandedReasoning.value[index] = !expandedReasoning.value[index];
};

const props = defineProps<{
  messages: Message[];
  isTyping: boolean;
  themeMode?: string;
}>();

const emit = defineEmits<{
  (e: 'send', content: string): void;
  (e: 'clear'): void;
}>();

const input = ref('');
const messagesEndRef = ref<HTMLElement | null>(null);
const inputRef = ref<HTMLTextAreaElement | null>(null);

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
  }

  // Auto-expand reasoning when it starts
  newMessages.forEach((msg, index) => {
    if (msg.role === 'agent' && msg.isThinking && expandedReasoning.value[index] === undefined) {
       expandedReasoning.value[index] = true;
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
        <p class="text-lg">开始和 Hikari 对话吧～</p>
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
                  {{ msg.toolStatus === 'running' ? '正在调用工具...' : (msg.toolStatus === 'success' ? '调用成功' : '调用失败') }}
                </span>
                
                <span v-if="msg.toolName" class="text-xs px-1.5 py-0.5 rounded bg-gray-100 text-gray-500 border border-gray-200">
                  {{ msg.toolName }}
                </span>
              </div>
              
              <!-- Tool Details Toggle -->
              <div v-if="msg.toolStatus !== 'running'" class="px-3 pb-2 flex justify-end">
                <button 
                  @click="toggleTool(index)"
                  class="text-[10px] flex items-center space-x-1 text-gray-400 hover:text-gray-600 transition-colors"
                >
                  <span>{{ expandedTools[index] ? '收起详情' : '查看详情' }}</span>
                  <component :is="expandedTools[index] ? ChevronDown : ChevronRight" :size="12" />
                </button>
              </div>

              <!-- Tool Details Content -->
              <div v-if="expandedTools[index]" class="border-t border-gray-100 bg-gray-50/50 p-3 text-xs space-y-2">
                <div>
                  <div class="font-semibold text-gray-500 mb-1">输入参数:</div>
                  <div class="bg-gray-100 rounded p-2 font-mono text-gray-600 break-all whitespace-pre-wrap">{{ msg.toolArgs }}</div>
                </div>
                <div v-if="msg.toolResult">
                  <div class="font-semibold text-gray-500 mb-1">执行结果:</div>
                  <div class="bg-white border border-gray-200 rounded p-2 font-mono text-gray-600 max-h-40 overflow-y-auto break-all whitespace-pre-wrap">{{ msg.toolResult }}</div>
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
                          {{ msg.isThinking ? '正在深度思考...' : '深度思考过程' }}
                       </span>
                    </div>
                    <component :is="expandedReasoning[index] ? ChevronDown : ChevronRight" :size="14" class="text-gray-400" />
                 </div>
                 
                 <!-- Content -->
                 <div v-if="expandedReasoning[index]" class="px-3 py-2 border-t border-gray-100/50 bg-gray-50/30 text-xs text-gray-600">
                    <div class="markdown-body" v-html="md.render(msg.reasoning)"></div>
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
      <div class="flex items-center space-x-3 bg-white rounded-xl border border-gray-200 px-2 py-2 shadow-sm focus-within:ring-2 focus-within:ring-pink-500/20 focus-within:border-pink-500 transition-all">
        <button
          @click="emit('clear')"
          class="p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-lg transition-all flex-shrink-0"
          title="清除对话"
        >
          <Trash2 :size="20" />
        </button>
        
        <div class="flex-1 relative flex items-center">
          <textarea
            ref="inputRef"
            v-model="input"
            @keydown="handleKeyDown"
            placeholder="输入消息... (Enter 发送)"
            class="w-full bg-transparent text-sm resize-none focus:outline-none placeholder-gray-400 py-2 max-h-[120px]"
            rows="1"
            style="min-height: 24px;"
          />
        </div>
        
        <button
          @click="handleSend"
          :disabled="!input.trim() || isTyping"
          class="p-2 rounded-lg transition-all flex items-center justify-center flex-shrink-0"
          :class="!input.trim() || isTyping ? 'bg-gray-100 text-gray-400 cursor-not-allowed' : 'bg-gradient-to-r from-pink-500 to-pink-600 text-white shadow-md shadow-pink-500/20 hover:shadow-lg hover:scale-105 active:scale-95'"
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
