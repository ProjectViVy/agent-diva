<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import NormalMode from "./components/NormalMode.vue";

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
    content: '你好呀，主人～ 💕\n\n我是 Hikari (光)。我会一直陪着你，关注你的一举一动。无论你想做什么，都可以告诉我哦～',
    timestamp: Date.now(),
    emotion: 'happy'
  }
]);
const isTyping = ref(false);
const connectionStatus = ref<'connected' | 'error' | 'connecting'>('connected');
const currentEmotion = ref('happy');

// Config state
const config = ref({
  apiBase: "https://api.deepseek.com/v1",
  apiKey: "",
  model: "deepseek-chat"
});

const savedModels = ref<SavedModel[]>([]);

const unlisteners: UnlistenFn[] = [];

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
                lastMsg.content = `[Mock Response] I received: "${content}"\n(Tauri API not available in browser)`;
                lastMsg.isStreaming = false;
                isTyping.value = false;
            }
        }, 1000);
        return;
    }

    await invoke("send_message", { message: content });
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
      content: `Error: ${error}`, 
      timestamp: Date.now() 
    });
    
    isTyping.value = false;
  }
}

const clearMessages = () => {
  messages.value = [
    {
      role: 'agent',
      content: '聊天记录已清除～ 让我们重新开始吧！💕',
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
            content: "[Mock] Configuration updated successfully (Browser Mode).", 
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
      content: "Configuration updated successfully.", 
      timestamp: Date.now() 
    });
  } catch (error) {
    alert(`Failed to save config: ${error}`);
  }
}

onMounted(async () => {
  if (!isTauri()) {
      console.log("Running in browser mode - Tauri listeners skipped");
      return;
  }

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
  unlisteners.push(await listen<string>("agent-tool-delta", (event) => {
    // Optional: show tool usage in UI
  }));

  unlisteners.push(await listen<string>("agent-tool-start", (event) => {
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && (lastMsg.content === '') && lastMsg.isStreaming) {
      messages.value.pop();
    } else if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.isStreaming = false;
    }

    let toolName = 'Unknown Tool';
    let toolArgs = '';

    try {
        // Try to parse "Using tool <name>: <args>" format
        const match = event.payload.match(/^Using tool ([^:]+): (.*)$/);
        if (match) {
            toolName = match[1];
            toolArgs = match[2];
        } else {
            // Fallback
            toolName = 'Tool';
            toolArgs = event.payload;
        }
    } catch (e) {
        toolArgs = event.payload;
    }

    messages.value.push({ 
      role: 'tool', 
      content: '正在调用工具...', 
      timestamp: Date.now(),
      toolName,
      toolArgs,
      toolStatus: 'running'
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

  unlisteners.push(await listen<string>("agent-tool-end", (event) => {
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.content === '' && lastMsg.isStreaming) {
      messages.value.pop();
    }
    
    // Find the last running tool message
    // Iterate backwards to find the corresponding tool start message
    let toolMsgIndex = -1;
    for (let i = messages.value.length - 1; i >= 0; i--) {
        if (messages.value[i].role === 'tool' && messages.value[i].toolStatus === 'running') {
            toolMsgIndex = i;
            break;
        }
    }

    if (toolMsgIndex !== -1) {
        // We found a matching running tool message
        messages.value[toolMsgIndex].toolStatus = 'success';
        messages.value[toolMsgIndex].content = '调用成功！';
        
        // Parse result
        let result = event.payload;
        try {
             // Try to parse "Tool <name> finished: <result>" format
            const match = event.payload.match(/^Tool ([^ ]+) finished: (.*)$/);
            if (match) {
                result = match[2];
            }
        } catch(e) {}
        
        messages.value[toolMsgIndex].toolResult = result;
    } else {
        // If no matching start message found (shouldn't happen usually), add a new one
        messages.value.push({ 
          role: 'tool', 
          content: `✅ ${event.payload}`, 
          timestamp: Date.now(),
          toolStatus: 'success'
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
            if (newLastMsg.role === 'system' && newLastMsg.content === `Error: ${event.payload}`) {
                return; // Skip duplicate
            }
        }
    }

    messages.value.push({ 
      role: 'system', 
      content: `Error: ${event.payload}`, 
      timestamp: Date.now() 
    });
    isTyping.value = false;
  }));

  // Listen for external hook messages
  unlisteners.push(await listen<string>("external-message", (event) => {
    messages.value.push({ 
      role: 'system', 
      content: `[Hook]: ${event.payload}`, 
      timestamp: Date.now() 
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
      :saved-models="savedModels"
      @send="sendMessage"
      @clear="clearMessages"
      @save-config="saveConfig"
      @update-saved-models="updateSavedModels"
    />
  </div>
</template>

<style>
/* Global resets if needed, but tailwind handles most */
</style>
