<script setup lang="ts">
import { ref, computed } from 'vue';
import { Menu, MessageSquare, Settings, Heart, Minus, X, Server, Check } from 'lucide-vue-next';
import { getCurrentWindow } from '@tauri-apps/api/window';
import ChatView from './ChatView.vue';
import SettingsView from './SettingsView.vue';

interface Message {
  role: 'user' | 'agent' | 'system' | 'tool';
  content: string;
  isStreaming?: boolean;
  timestamp?: number;
  emotion?: string;
}

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

const props = defineProps<{
  messages: Message[];
  isTyping: boolean;
  connectionStatus?: 'connected' | 'error' | 'connecting';
  currentEmotion?: string;
  config?: {
    apiBase: string;
    apiKey: string;
    model: string;
  };
  savedModels?: SavedModel[];
}>();

const emit = defineEmits<{
  (e: 'send', content: string): void;
  (e: 'clear'): void;
  (e: 'toggle-sidebar'): void;
  (e: 'save-config', config: { apiBase: string; apiKey: string; model: string }): void;
  (e: 'update-saved-models', models: SavedModel[]): void;
}>();

const activeTab = ref<'chat' | 'settings'>('chat');
const sidebarOpen = ref(false);
const soulSidebarOpen = ref(false);
const themeMode = ref('love'); // Default to love theme
const isModelDropdownOpen = ref(false);

const handleSaveConfig = (newConfig: any) => {
  emit('save-config', newConfig);
};

const handleUpdateSavedModels = (models: SavedModel[]) => {
  emit('update-saved-models', models);
};

const selectSavedModel = (model: SavedModel) => {
  // Switch to this model
  emit('save-config', {
    apiBase: model.apiBase,
    apiKey: model.apiKey,
    model: model.model
  });
  isModelDropdownOpen.value = false;
};

const toggleSidebar = () => {
  sidebarOpen.value = !sidebarOpen.value;
  emit('toggle-sidebar');
};

const toggleSoulSidebar = () => {
  soulSidebarOpen.value = !soulSidebarOpen.value;
};

const minimizeWindow = async () => {
  try {
    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      await getCurrentWindow().minimize();
    } else {
      console.warn('minimizeWindow mocked in browser');
    }
  } catch (e) {
    console.error('Failed to minimize window', e);
  }
};

const closeWindow = async () => {
  try {
    if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
      await getCurrentWindow().close();
    } else {
      console.warn('closeWindow mocked in browser');
    }
  } catch (e) {
    console.error('Failed to close window', e);
  }
};

const hearts = [
  { left: '8%', top: '12%', size: 18, opacity: 0.35, delay: 0 },
  { left: '20%', top: '70%', size: 12, opacity: 0.25, delay: 0.6 },
  { left: '34%', top: '28%', size: 22, opacity: 0.3, delay: 1.2 },
  { left: '48%', top: '55%', size: 14, opacity: 0.2, delay: 0.9 },
  { left: '62%', top: '18%', size: 26, opacity: 0.35, delay: 0.3 },
  { left: '72%', top: '72%', size: 16, opacity: 0.25, delay: 1.6 },
  { left: '84%', top: '40%', size: 20, opacity: 0.3, delay: 0.8 },
  { left: '90%', top: '15%', size: 12, opacity: 0.22, delay: 1.1 },
];

const emotionConfig: Record<string, { emoji: string; label: string }> = {
  happy: { emoji: '😊', label: '开心' },
  sad: { emoji: '😢', label: '难过' },
  clingy: { emoji: '🥺', label: '粘人' },
  jealous: { emoji: '😤', label: '吃醋' },
  angry: { emoji: '😠', label: '生气' },
  normal: { emoji: '🙂', label: '平静' },
};

const currentConfig = emotionConfig[props.currentEmotion || 'normal'] || emotionConfig['normal'];
</script>

<template>
  <div class="app-shell w-full h-full flex flex-col overflow-hidden rounded-xl relative" :class="`theme-${themeMode}`">
    <!-- Love Hearts Background -->
    <div v-if="themeMode === 'love'" class="love-hearts">
      <span
        v-for="(h, i) in hearts"
        :key="i"
        class="love-heart"
        :style="{
          left: h.left,
          top: h.top,
          width: `${h.size}px`,
          height: `${h.size}px`,
          opacity: h.opacity,
          animationDelay: `${h.delay}s`,
        }"
      />
    </div>

    <!-- Titlebar -->
    <header 
      class="app-titlebar h-12 flex items-center justify-between px-4 relative z-50 border-b drag-region"
    >
      <div class="flex items-center space-x-3">
        <!-- Sidebar Toggle -->
        <button 
          @click="toggleSidebar"
          class="p-1.5 rounded-md transition-colors no-drag"
          :class="sidebarOpen ? 'bg-gray-200 text-gray-800' : 'text-gray-500 hover:bg-gray-200'"
        >
          <Menu :size="18" />
        </button>

        <!-- Emotion Indicator -->
        <div 
          class="app-emotion w-8 h-8 rounded-full flex items-center justify-center text-lg shadow-sm border animate-pulse-slow"
        >
          {{ currentConfig.emoji }}
        </div>
        
        <div class="flex flex-col">
          <h1 class="text-sm font-bold text-gray-800 leading-tight">
            Hikari
          </h1>
          <div class="flex items-center space-x-1.5 text-[10px] text-gray-500 leading-tight">
            <span class="app-badge px-1.5 rounded-full">
              {{ currentConfig.label }}
            </span>
            <span class="flex items-center space-x-1">
              <div 
                class="w-1.5 h-1.5 rounded-full"
                :class="{
                  'bg-green-500': connectionStatus === 'connected',
                  'bg-red-500': connectionStatus === 'error',
                  'bg-yellow-500 animate-pulse': connectionStatus === 'connecting'
                }"
              />
              <span>
                {{ connectionStatus === 'connected' ? '在线' : connectionStatus === 'error' ? '离线' : '连接中' }}
              </span>
            </span>
          </div>
        </div>

        <!-- Provider Selector Button -->
        <div class="relative no-drag ml-4">
          <button 
            v-if="config"
            @click="isModelDropdownOpen = !isModelDropdownOpen"
            class="flex items-center space-x-2 px-2 py-1 bg-gray-50 hover:bg-white border border-gray-200/50 hover:border-pink-200 rounded-lg transition-all text-xs text-gray-600 hover:text-pink-600 shadow-sm group"
            title="切换模型"
          >
            <Server :size="12" class="text-gray-400 group-hover:text-pink-500" />
            <span class="max-w-[100px] truncate font-medium">{{ config.model || 'Select Model' }}</span>
          </button>
          
          <!-- Dropdown -->
          <div v-if="isModelDropdownOpen" class="absolute top-full left-0 mt-1 w-48 bg-white rounded-lg shadow-xl border border-gray-100 overflow-hidden z-[100] animate-in fade-in zoom-in duration-100">
             <div class="py-1 max-h-60 overflow-y-auto">
                <div v-if="savedModels && savedModels.length > 0">
                    <button
                        v-for="model in savedModels"
                        :key="model.id"
                        @click="selectSavedModel(model)"
                        class="w-full text-left px-3 py-2 text-xs hover:bg-pink-50 flex items-center justify-between group"
                        :class="config?.model === model.model ? 'text-pink-600 font-medium' : 'text-gray-600'"
                    >
                        <span class="truncate">{{ model.displayName }}</span>
                        <Check v-if="config?.model === model.model" :size="12" class="text-pink-500" />
                    </button>
                </div>
                <div v-else class="px-3 py-4 text-center text-gray-400 text-[10px]">
                    暂无已保存模型<br>请在设置中添加
                </div>
                
                <div class="border-t border-gray-100 mt-1 pt-1">
                    <button 
                        @click="activeTab = 'settings'; isModelDropdownOpen = false"
                        class="w-full text-left px-3 py-2 text-xs text-gray-500 hover:text-gray-800 hover:bg-gray-50 flex items-center"
                    >
                        <Settings :size="12" class="mr-2" />
                        管理模型...
                    </button>
                </div>
             </div>
          </div>
          
          <!-- Overlay to close -->
          <div v-if="isModelDropdownOpen" class="fixed inset-0 z-[90]" @click="isModelDropdownOpen = false"></div>
        </div>
      </div>

      <div class="flex items-center space-x-1 no-drag">
        <!-- Tabs -->
        <nav class="flex space-x-1 p-0.5 rounded-lg mr-4">
          <button
            @click="activeTab = 'chat'"
            class="flex items-center space-x-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-200"
            :class="activeTab === 'chat' ? 'text-green-600 bg-gray-200/50' : 'text-gray-500 hover:text-gray-800 hover:bg-gray-200/30'"
          >
            <MessageSquare :size="16" />
            <span v-if="messages.length > 0" class="bg-red-500 text-white text-[10px] px-1 rounded-full min-w-[16px] text-center ml-1">
              {{ messages.length }}
            </span>
          </button>
          <button
            @click="activeTab = 'settings'"
            class="flex items-center space-x-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-all duration-200"
            :class="activeTab === 'settings' ? 'text-green-600 bg-gray-200/50' : 'text-gray-500 hover:text-gray-800 hover:bg-gray-200/30'"
          >
            <Settings :size="16" />
          </button>
        </nav>
        
        <!-- Window Controls -->
        <div class="flex items-center space-x-1">
          <button
            @click="toggleSoulSidebar"
            class="p-1.5 rounded-md transition-colors no-drag mr-2"
            :class="soulSidebarOpen ? 'bg-pink-100 text-pink-600' : 'text-gray-500 hover:bg-pink-50 hover:text-pink-500'"
            title="情感状态"
          >
            <Heart :size="16" />
          </button>

          <button
            @click="minimizeWindow"
            class="p-1.5 text-gray-500 hover:text-gray-800 hover:bg-gray-200/50 rounded-md transition-colors"
            title="最小化"
          >
            <Minus :size="16" />
          </button>
          <button
            @click="closeWindow"
            class="p-1.5 text-gray-500 hover:text-white hover:bg-red-500 rounded-md transition-colors"
            title="关闭"
          >
            <X :size="16" />
          </button>
        </div>
      </div>
    </header>

    <!-- Main Content -->
    <main class="flex-1 overflow-hidden relative z-10">
      <div v-if="activeTab === 'chat'" class="h-full">
        <ChatView
          :messages="messages"
          :is-typing="isTyping"
          :theme-mode="themeMode"
          @send="(content) => emit('send', content)"
          @clear="emit('clear')"
        />
      </div>
      <div v-else class="h-full">
        <SettingsView
          v-if="config"
          :config="config"
          :saved-models="savedModels"
          @save="(newConfig) => emit('save-config', newConfig)"
          @update-saved-models="handleUpdateSavedModels"
        />
        <div v-else class="h-full flex items-center justify-center text-gray-500">
          Loading configuration...
        </div>
      </div>
    </main>

  </div>
</template>

<style scoped>
/* Scoped styles */
</style>
