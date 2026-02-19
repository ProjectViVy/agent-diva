<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

interface Message {
  role: 'user' | 'agent' | 'system' | 'tool';
  content: string;
  isStreaming?: boolean;
}

const messages = ref<Message[]>([]);
const inputMessage = ref("");
const messagesContainer = ref<HTMLElement | null>(null);
const isProcessing = ref(false);
const showConfig = ref(false);

// Config state
const config = ref({
  apiBase: "https://api.deepseek.com/v1",
  apiKey: "",
  model: "deepseek-chat"
});

const unlisteners: UnlistenFn[] = [];

function scrollToBottom() {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
    }
  });
}

async function sendMessage() {
  if (!inputMessage.value.trim() || isProcessing.value) return;

  const userMsg = inputMessage.value;
  messages.value.push({ role: 'user', content: userMsg });
  inputMessage.value = "";
  scrollToBottom();

  isProcessing.value = true;
  
  // Create a placeholder for the agent response
  messages.value.push({ role: 'agent', content: '', isStreaming: true });

  try {
    await invoke("send_message", { message: userMsg });
  } catch (error) {
    console.error("Failed to send message:", error);
    messages.value.push({ role: 'system', content: `Error: ${error}` });
    
    // Check if error is about configuration
    if (typeof error === 'string' && error.includes("configured")) {
      showConfig.value = true;
    }
    
    isProcessing.value = false;
  }
}

async function saveConfig() {
  try {
    await invoke("update_config", {
      apiBase: config.value.apiBase || null,
      apiKey: config.value.apiKey || null,
      model: config.value.model || null
    });
    showConfig.value = false;
    messages.value.push({ role: 'system', content: "Configuration updated successfully." });
    scrollToBottom();
  } catch (error) {
    alert(`Failed to save config: ${error}`);
  }
}

onMounted(async () => {
  // Listen for streaming text delta
  unlisteners.push(await listen<string>("agent-response-delta", (event) => {
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.content += event.payload;
      scrollToBottom();
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
      isProcessing.value = false;
      scrollToBottom();
    }
  }));

  // Listen for tool usage
  unlisteners.push(await listen<string>("agent-tool-delta", (event) => {
    // Show tool preparation/thinking progress
    // We append this to a temporary 'tool-thinking' message if it exists, or create one
    // But since we want to avoid UI clutter, let's just make the "Agent" message show "Thinking..."
    // actually, let's append to the agent message but mark it differently?
    // Or just ignore the content but ensure the spinner is active.
    
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
       // If the last message is streaming agent content, we can append a status indicator
       // But text delta and tool delta might be mixed.
       // Let's just keep the spinner active.
       if (!lastMsg.content.endsWith("...")) {
           // lastMsg.content += "."; 
       }
    } else {
       // If no streaming message, create one
       messages.value.push({ role: 'agent', content: '(Preparing tool...)', isStreaming: true });
    }
  }));

  unlisteners.push(await listen<string>("agent-tool-start", (event) => {
    const lastMsg = messages.value[messages.value.length - 1];
    // If the last message was an empty streaming agent message, remove it
    if (lastMsg && lastMsg.role === 'agent' && (lastMsg.content === '' || lastMsg.content === '(Preparing tool...)') && lastMsg.isStreaming) {
      messages.value.pop();
    } else if (lastMsg && lastMsg.role === 'agent' && lastMsg.isStreaming) {
      lastMsg.isStreaming = false;
    }

    messages.value.push({ role: 'tool', content: `🛠️ ${event.payload}` });
    messages.value.push({ role: 'agent', content: '', isStreaming: true });
    scrollToBottom();
  }));

  unlisteners.push(await listen<string>("agent-tool-end", (event) => {
    // Remove the placeholder agent message added in tool-start
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg && lastMsg.role === 'agent' && lastMsg.content === '' && lastMsg.isStreaming) {
      messages.value.pop();
    }

    messages.value.push({ role: 'tool', content: `✅ ${event.payload}` });
    // Add new placeholder for subsequent agent response
    messages.value.push({ role: 'agent', content: '', isStreaming: true });
    scrollToBottom();
  }));

  // Listen for errors
  unlisteners.push(await listen<string>("agent-error", (event) => {
    messages.value.push({ role: 'system', content: `Error: ${event.payload}` });
    isProcessing.value = false;
    scrollToBottom();
  }));

  // Listen for external hook messages
  unlisteners.push(await listen<string>("external-message", (event) => {
    messages.value.push({ role: 'system', content: `[Hook]: ${event.payload}` });
    scrollToBottom();
  }));
});

onUnmounted(() => {
  unlisteners.forEach(fn => fn());
});
</script>

<template>
  <div class="chat-container">
    <header>
      <h1>Agent Diva GUI</h1>
      <button class="config-btn" @click="showConfig = true">⚙️</button>
    </header>
    
    <div class="messages" ref="messagesContainer">
      <div v-for="(msg, index) in messages" :key="index" :class="['message-wrapper', msg.role]">
        <div class="message-content">
          <span v-if="msg.role === 'agent' && msg.content === '' && msg.isStreaming" class="typing-indicator">...</span>
          <span v-else>{{ msg.content }}</span>
        </div>
      </div>
    </div>
    
    <div class="input-area">
      <input 
        v-model="inputMessage" 
        @keyup.enter="sendMessage"
        :disabled="isProcessing"
        placeholder="Type a message to Agent Diva..."
      />
      <button @click="sendMessage" :disabled="isProcessing">
        {{ isProcessing ? 'Thinking...' : 'Send' }}
      </button>
    </div>

    <!-- Config Modal -->
    <div v-if="showConfig" class="modal-overlay">
      <div class="modal">
        <h2>Configuration</h2>
        <div class="form-group">
          <label>API Base URL</label>
          <input v-model="config.apiBase" placeholder="https://api.deepseek.com/v1" />
        </div>
        <div class="form-group">
          <label>API Key</label>
          <input v-model="config.apiKey" type="password" placeholder="sk-..." />
        </div>
        <div class="form-group">
          <label>Model</label>
          <input v-model="config.model" placeholder="deepseek-chat" />
        </div>
        <div class="modal-actions">
          <button @click="showConfig = false" class="cancel-btn">Cancel</button>
          <button @click="saveConfig" class="save-btn">Save</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
  box-sizing: border-box;
  font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
  position: relative;
}

header {
  display: flex;
  justify-content: center;
  align-items: center;
  margin-bottom: 20px;
  border-bottom: 1px solid #eee;
  padding-bottom: 10px;
  position: relative;
}

h1 {
  margin: 0;
  color: #333;
}

.config-btn {
  position: absolute;
  right: 0;
  background: none;
  border: none;
  font-size: 1.5em;
  padding: 5px;
  cursor: pointer;
  color: #666;
}

.config-btn:hover {
  background: none;
  color: #333;
}

/* ... existing styles ... */
.messages {
  flex: 1;
  overflow-y: auto;
  margin-bottom: 20px;
  display: flex;
  flex-direction: column;
  gap: 15px;
  padding: 10px;
  background-color: #f9f9f9;
  border-radius: 8px;
  border: 1px solid #ddd;
}

.message-wrapper {
  display: flex;
  width: 100%;
}

.message-wrapper.user {
  justify-content: flex-end;
}

.message-wrapper.agent {
  justify-content: flex-start;
}

.message-wrapper.system, .message-wrapper.tool {
  justify-content: center;
}

.message-content {
  padding: 12px 16px;
  border-radius: 12px;
  max-width: 80%;
  word-wrap: break-word;
  line-height: 1.5;
  white-space: pre-wrap;
}

.user .message-content {
  background-color: #007bff;
  color: white;
  border-bottom-right-radius: 2px;
}

.agent .message-content {
  background-color: #e9ecef;
  color: #333;
  border-bottom-left-radius: 2px;
}

.system .message-content {
  background-color: #fff3cd;
  color: #856404;
  font-size: 0.9em;
  font-style: italic;
  border: 1px solid #ffeeba;
}

.tool .message-content {
  background-color: #e2e3e5;
  color: #383d41;
  font-family: monospace;
  font-size: 0.85em;
  border: 1px solid #d6d8db;
}

.input-area {
  display: flex;
  gap: 10px;
}

input {
  flex: 1;
  padding: 12px;
  border-radius: 6px;
  border: 1px solid #ccc;
  font-size: 16px;
}

input:focus {
  outline: none;
  border-color: #007bff;
}

input:disabled {
  background-color: #eee;
  cursor: not-allowed;
}

button {
  padding: 0 24px;
  background-color: #007bff;
  color: white;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 16px;
  font-weight: 600;
  transition: background-color 0.2s;
}

button:hover {
  background-color: #0056b3;
}

button:disabled {
  background-color: #6c757d;
  cursor: not-allowed;
}

.typing-indicator {
  animation: pulse 1.5s infinite;
}

@keyframes pulse {
  0% { opacity: 0.4; }
  50% { opacity: 1; }
  100% { opacity: 0.4; }
}

/* Modal Styles */
.modal-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 100;
}

.modal {
  background-color: white;
  padding: 20px;
  border-radius: 8px;
  width: 90%;
  max-width: 400px;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
}

.modal h2 {
  margin-top: 0;
  margin-bottom: 20px;
  text-align: center;
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  margin-bottom: 5px;
  font-weight: 600;
  font-size: 0.9em;
}

.form-group input {
  width: 100%;
  box-sizing: border-box;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 20px;
}

.cancel-btn {
  background-color: #6c757d;
}

.cancel-btn:hover {
  background-color: #5a6268;
}

.save-btn {
  background-color: #28a745;
}

.save-btn:hover {
  background-color: #218838;
}
</style>
