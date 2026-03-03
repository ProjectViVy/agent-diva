<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import NormalMode from "./components/NormalMode.vue";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

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
}

interface ToolStartPayload {
  name: string;
  args_preview?: string;
  call_id?: string | null;
}

interface ToolFinishPayload {
  name: string;
  result: string;
  is_error?: boolean;
  call_id?: string | null;
}

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

const messages = ref<Message[]>([
  {
    role: 'agent',
    content: t('app.welcome'),
    timestamp: Date.now(),
    emotion: 'happy'
  }
]);
const isTyping = ref(false);
const connectionStatus = ref<'connected' | 'error' | 'connecting'>('connected');
const currentEmotion = ref('happy');
const suppressNextStopError = ref(false);
const currentChannel = ref('gui');
const currentChatId = ref(generateChatId());

// Config state
const config = ref({
  apiBase: "https://api.deepseek.com/v1",
  apiKey: "",
  model: "deepseek-chat"
});

const toolsConfig = ref({
  web: {
    search: {
      provider: 'brave',
      enabled: true,
      api_key: '',
      max_results: 5
    },
    fetch: {
      enabled: true
    }
  }
});

const savedModels = ref<SavedModel[]>([]);

const unlisteners: UnlistenFn[] = [];

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

function generateChatId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `chat-${crypto.randomUUID()}`;
  }
  return `chat-${Date.now()}-${Math.floor(Math.random() * 100000)}`;
}

// Load saved models from localStorage
onMounted(() => {
  const storedModels = localStorage.getItem('agent-diva-saved-models');
  if (storedModels) {
    try {
      savedModels.value = JSON.parse(storedModels);
    } catch (e) {
      console.error("Failed to parse saved models", e);
    }
  }
});

// Save models to localStorage whenever they change
watch(savedModels, (newVal) => {
  localStorage.setItem('agent-diva-saved-models', JSON.stringify(newVal));
}, { deep: true });

function updateSavedModels(models: SavedModel[]) {
  savedModels.value = models;
}

async function sendMessage(content: string) {
  if (!content.trim() || isTyping.value) return;
  if (content.trim() === '/stop') {
    await stopMessage();
    return;
  }

  console.log('[App] Sending message with current config:', {
      ...config.value,
      apiKey: config.value.apiKey ? `${config.value.apiKey.substring(0, 8)}...` : 'undefined'
  });

  const userMsg: Message = { 
    role: 'user', 
    content: content, 
    timestamp: Date.now() 
  };
  messages.value.push(userMsg);
  
  isTyping.value = true;
  suppressNextStopError.value = false;
  
  // Create a placeholder for the agent response
  messages.value.push({ 
    role: 'agent', 
    content: '', 
    isStreaming: true, 
    timestamp: Date.now(),
    emotion: currentEmotion.value
  });

  try {
    if (!isTauri()) {
        console.warn('Running in browser, mocking sendMessage');
        setTimeout(() => {
            const lastMsg = messages.value[messages.value.length - 1];
            if (lastMsg && lastMsg.role === 'agent') {
                lastMsg.content = t('app.mockResponse', { content });
                lastMsg.isStreaming = false;
                isTyping.value = false;
            }
        }, 1000);
        return;
    }

    await invoke("send_message", {
      message: content,
      channel: currentChannel.value,
      chatId: currentChatId.value,
    });
  } catch (error) {
    console.error("Failed to send message:", error);
    
    // Remove the placeholder agent message
    if (messages.value.length > 0) {
        const lastMsg = messages.value[messages.value.length - 1];
        if (lastMsg.role === 'agent' && lastMsg.isStreaming) {
            messages.value.pop();
        }
    }

    messages.value.push({ 
      role: 'system', 
      content: `${t('app.errorPrefix')}${error}`, 
      timestamp: Date.now() 
    });
    
    isTyping.value = false;
  }
}

async function stopMessage() {
  if (!isTyping.value) return;

  try {
    if (!isTauri()) {
      const lastMsg = messages.value[messages.value.length - 1];
      if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
        lastMsg.isStreaming = false;
        lastMsg.isThinking = false;
      }
      isTyping.value = false;
      messages.value.push({
        role: 'system',
        content: `[Mock] ${t('app.stopped')}`,
        timestamp: Date.now()
      });
      return;
    }

    await invoke("stop_generation", {
      channel: currentChannel.value,
      chatId: currentChatId.value,
    });
    suppressNextStopError.value = true;
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.isStreaming = false;
      lastMsg.isThinking = false;
    }
    isTyping.value = false;
    messages.value.push({
      role: 'system',
      content: t('app.stopRequested'),
      timestamp: Date.now()
    });
  } catch (error) {
    messages.value.push({
      role: 'system',
      content: t('app.stopFailed', { error }),
      timestamp: Date.now()
    });
  }
}

const clearMessages = () => {
  currentChatId.value = generateChatId();
  messages.value = [
    {
      role: 'agent',
      content: t('app.cleared'),
      timestamp: Date.now(),
      emotion: 'happy'
    }
  ];
};

async function saveConfig(newConfig: typeof config.value) {
  try {
    // Update local config ref
    config.value = { ...newConfig };
    
    console.log('[App] Saving config:', { 
        ...newConfig, 
        apiKey: newConfig.apiKey ? `${newConfig.apiKey.substring(0, 8)}...` : 'undefined' 
    });

    if (!isTauri()) {
        console.warn('Running in browser, mocking saveConfig');
        messages.value.push({ 
            role: 'system', 
            content: "[Mock] " + t('app.configUpdated'), 
            timestamp: Date.now() 
        });
        return;
    }

    await invoke("update_config", {
      apiBase: newConfig.apiBase || null,
      apiKey: newConfig.apiKey || null,
      model: newConfig.model || null
    });
    
    messages.value.push({ 
      role: 'system', 
      content: t('app.configUpdated'), 
      timestamp: Date.now() 
    });
  } catch (error) {
    alert(t('app.configUpdateError', { error }));
  }
}

async function saveToolsConfig(newToolsConfig: typeof toolsConfig.value) {
  try {
    toolsConfig.value = JSON.parse(JSON.stringify(newToolsConfig));
    if (!isTauri()) {
      messages.value.push({
        role: 'system',
        content: "[Mock] " + t('app.configUpdated'),
        timestamp: Date.now()
      });
      return;
    }

    await invoke("update_tools_config", {
      tools: newToolsConfig
    });

    messages.value.push({
      role: 'system',
      content: t('app.configUpdated'),
      timestamp: Date.now()
    });
  } catch (error) {
    alert(t('app.configUpdateError', { error }));
  }
}

async function checkHealth() {
  if (!isTauri()) return;
  
  try {
    const isHealthy = await invoke<boolean>("check_health");
    connectionStatus.value = isHealthy ? 'connected' : 'error';
  } catch (e) {
    console.error("Health check failed:", e);
    connectionStatus.value = 'error';
  }
}

onMounted(async () => {
  if (!isTauri()) {
      console.log("Running in browser mode - Tauri listeners skipped");
      return;
  }

  try {
    await invoke("start_background_stream");
  } catch (e) {
    console.warn("Failed to start background stream:", e);
  }

  try {
    const fetchedTools = await invoke<typeof toolsConfig.value>("get_tools_config");
    toolsConfig.value = fetchedTools;
  } catch (e) {
    console.warn("Failed to load tools config:", e);
  }

  // Initial health check and polling
  await checkHealth();
  const healthInterval = setInterval(checkHealth, 5000);
  
  // Register cleanup
  onUnmounted(() => {
    clearInterval(healthInterval);
  });

  // Listen for streaming text delta
  unlisteners.push(await listen<string>("agent-response-delta", (event) => {
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.content += event.payload;
    }
  }));

  // Listen for reasoning delta
  unlisteners.push(await listen<string>("agent-reasoning-delta", (event) => {
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      if (!lastMsg.reasoning) {
        lastMsg.reasoning = "";
      }
      lastMsg.reasoning += event.payload;
      lastMsg.isThinking = true;
    }
  }));

  // Listen for completion
  unlisteners.push(await listen<string>("agent-response-complete", (event) => {
    suppressNextStopError.value = false;
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      if (!lastMsg.content && event.payload) {
         lastMsg.content = event.payload;
      }
      lastMsg.isStreaming = false;
      lastMsg.isThinking = false;
      isTyping.value = false;
    }
  }));

  // Listen for tool usage
  unlisteners.push(await listen<string>("agent-tool-delta", () => {
    // Optional: show tool usage in UI
  }));

  unlisteners.push(await listen<ToolStartPayload>("agent-tool-start", (event) => {
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && (lastMsg.content === '') && lastMsg.isStreaming) {
      messages.value.pop();
    } else if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.isStreaming = false;
    }

    const payload = event.payload || ({} as ToolStartPayload);
    const toolName = payload.name || t('app.unknownTool');
    const toolArgs = payload.args_preview || '';

    messages.value.push({ 
      role: 'tool', 
      content: t('app.toolRunning'), 
      timestamp: Date.now(),
      toolName,
      toolArgs,
      toolStatus: 'running',
      toolCallId: payload.call_id || undefined
    });
    
    // Add placeholder for next agent response
    messages.value.push({ 
      role: 'agent', 
      content: '', 
      isStreaming: true, 
      timestamp: Date.now(),
      emotion: currentEmotion.value
    });
  }));

  unlisteners.push(await listen<ToolFinishPayload>("agent-tool-end", (event) => {
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.content === '' && lastMsg.isStreaming) {
      messages.value.pop();
    }

    const payload = event.payload || ({} as ToolFinishPayload);

    // Prefer matching by call_id to avoid cross-updates when multiple tools run.
    let toolMsgIndex = -1;
    if (payload.call_id) {
      toolMsgIndex = messages.value.findIndex(
        (msg) => msg.role === 'tool' && msg.toolStatus === 'running' && msg.toolCallId === payload.call_id
      );
    }
    if (toolMsgIndex === -1) {
      for (let i = messages.value.length - 1; i >= 0; i--) {
          if (messages.value[i].role === 'tool' && messages.value[i].toolStatus === 'running') {
              toolMsgIndex = i;
              break;
          }
      }
    }

    if (toolMsgIndex !== -1) {
        const isError = payload.is_error === true || payload.result?.startsWith('Error');
        messages.value[toolMsgIndex].toolStatus = isError ? 'error' : 'success';
        messages.value[toolMsgIndex].content = isError ? t('app.toolError') : t('app.toolSuccess');
        if (payload.name) {
          messages.value[toolMsgIndex].toolName = payload.name;
        }
        messages.value[toolMsgIndex].toolResult = payload.result || '';
    } else {
        // If no matching start message found, add a new entry.
        messages.value.push({
          role: 'tool',
          content: payload.is_error ? t('app.toolError') : t('app.toolSuccess'),
          timestamp: Date.now(),
          toolName: payload.name || t('app.unknownTool'),
          toolResult: payload.result || '',
          toolStatus: payload.is_error ? 'error' : 'success',
          toolCallId: payload.call_id || undefined
        });
    }

    messages.value.push({
      role: 'agent',
      content: '',
      isStreaming: true,
      timestamp: Date.now(),
      emotion: currentEmotion.value
    });
  }));

  // Listen for errors
  unlisteners.push(await listen<string>("agent-error", (event) => {
    if (suppressNextStopError.value && event.payload === "Generation stopped by user.") {
      suppressNextStopError.value = false;
      return;
    }
    
    // Check if the last message is a duplicate error message to debounce
    if (messages.value.length > 0) {
        const lastMsg = messages.value[messages.value.length - 1];
        
        // Remove streaming placeholder if exists
        if (lastMsg.role === 'agent' && lastMsg.isStreaming) {
            messages.value.pop();
        }
        
        // Re-check last message after pop
        if (messages.value.length > 0) {
            const newLastMsg = messages.value[messages.value.length - 1];
            if (newLastMsg.role === 'system' && newLastMsg.content === `${t('app.errorPrefix')}${event.payload}`) {
                return; // Skip duplicate
            }
        }
    }

    messages.value.push({ 
      role: 'system', 
      content: `${t('app.errorPrefix')}${event.payload}`, 
      timestamp: Date.now() 
    });
    isTyping.value = false;
  }));

  // Listen for external hook messages
  unlisteners.push(await listen<string>("external-message", (event) => {
    messages.value.push({ 
      role: 'system', 
      content: t('app.hookMessage', { message: event.payload }), 
      timestamp: Date.now() 
    });
  }));

  // Listen for background responses (e.g. scheduled cron executions)
  unlisteners.push(await listen<string>("agent-background-response", (event) => {
    messages.value.push({
      role: 'agent',
      content: event.payload,
      timestamp: Date.now(),
      emotion: currentEmotion.value
    });
  }));
});

onUnmounted(() => {
  unlisteners.forEach(fn => fn());
});
</script>

<template>
  <div class="w-screen h-screen overflow-hidden bg-transparent">
    <NormalMode
      :messages="messages"
      :is-typing="isTyping"
      :connection-status="connectionStatus"
      :current-emotion="currentEmotion"
      :config="config"
      :tools-config="toolsConfig"
      :saved-models="savedModels"
      @send="sendMessage"
      @clear="clearMessages"
      @stop="stopMessage"
      @save-config="saveConfig"
      @save-tools-config="saveToolsConfig"
      @update-saved-models="updateSavedModels"
    />
  </div>
</template>

<style>
/* Global resets if needed, but tailwind handles most */
</style>
