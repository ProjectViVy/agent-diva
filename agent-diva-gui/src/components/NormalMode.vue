<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue';
import {
  AlarmClock,
  Check,
  Heart,
  History,
  Menu,
  MessageSquare,
  Server,
  Settings,
  Trash2,
} from 'lucide-vue-next';
import ChatView from './ChatView.vue';
import SettingsView from './SettingsView.vue';
import CronTaskManagementView from './CronTaskManagementView.vue';
import AppDialogLayer from './AppDialogLayer.vue';
import AppToastLayer from './AppToastLayer.vue';
import { useI18n } from 'vue-i18n';
import type { FileAttachmentDto } from '../api/desktop';

const { t } = useI18n();

interface Message {
  role: 'user' | 'agent' | 'system' | 'tool';
  content: string;
  reasoning?: string;
  toolName?: string;
  toolArgs?: string;
  toolResult?: string;
  toolStatus?: 'running' | 'success' | 'error';
  toolCallId?: string;
  rawMeta?: Record<string, unknown>;
  isStreaming?: boolean;
  timestamp?: number;
  emotion?: string;
  attachments?: string[];
}

interface ChatDisplayPrefs {
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
}

type SettingsSubview =
  | 'dashboard'
  | 'general'
  | 'mcp'
  | 'skills'
  | 'providers'
  | 'channels'
  | 'network'
  | 'language'
  | 'about';

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}

interface AppConfigShape {
  provider: string;
  apiBase: string;
  apiKey: string;
  model: string;
}

interface ProviderConfigEntry {
  apiKey: string;
  apiBase: string;
  source: 'providers' | 'custom_providers';
}

interface ToolsConfigShape {
  web: {
    search: {
      provider: string;
      enabled: boolean;
      api_key: string;
      max_results: number;
    };
    fetch: {
      enabled: boolean;
    };
  };
}

interface Props {
  messages: Message[];
  isTyping: boolean;
  connectionStatus?: 'connected' | 'error' | 'connecting';
  currentEmotion?: string;
  config?: AppConfigShape;
  providerConfigs?: Record<string, ProviderConfigEntry>;
  toolsConfig?: ToolsConfigShape;
  savedModels?: SavedModel[];
  sessions?: { session_key: string; chat_id: string; snippet: string; timestamp: number }[];
  chatDisplayPrefs: ChatDisplayPrefs;
  saveConfigAction: (config: AppConfigShape) => Promise<void>;
  saveToolsConfigAction: (tools: ToolsConfigShape) => Promise<void>;
  saveChannelConfigAction: (channelName: string, channelConfig: Record<string, unknown>) => Promise<void>;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'send', content: string, attachments?: FileAttachmentDto[]): void;
  (e: 'clear'): void;
  (e: 'stop'): void;
  (e: 'toggle-sidebar'): void;
  (e: 'update-saved-models', models: SavedModel[]): void;
  (e: 'save-chat-display-prefs', prefs: ChatDisplayPrefs): void;
  (e: 'load-session', sessionKey: string): void;
  (e: 'delete-session', sessionKey: string): void;
}>();

type SidebarSection = 'chat' | 'settings' | 'console' | 'neuro' | 'cron';

const activeTab = ref<'chat' | 'settings'>('chat');
const activeMenu = ref<'console' | 'neuro' | 'cron' | null>(null);
const settingsInitialView = ref<SettingsSubview>('dashboard');
const sidebarOpen = ref(false);
const themeMode = ref('love');
const isModelDropdownOpen = ref(false);
const isHistoryDropdownOpen = ref(false);

const handleSessionSelect = (sessionKey: string) => {
  emit('load-session', sessionKey);
  isHistoryDropdownOpen.value = false;
};

const handleDeleteSession = (sessionKey: string) => {
  emit('delete-session', sessionKey);
  isHistoryDropdownOpen.value = false;
};

const handleUpdateSavedModels = (models: SavedModel[]) => {
  emit('update-saved-models', models);
};

const selectSavedModel = async (model: SavedModel) => {
  try {
    await props.saveConfigAction({
      provider: model.provider,
      apiBase: model.apiBase,
      apiKey: model.apiKey,
      model: model.model,
    });
  } catch (_) {
    return;
  }
  isModelDropdownOpen.value = false;
};

const isSavedModelSelected = (model: SavedModel) =>
  props.config?.provider === model.provider && props.config?.model === model.model;

const removeSavedModel = async (model: SavedModel, event: MouseEvent) => {
  event.stopPropagation();

  const nextModels = (props.savedModels || []).filter((entry) => entry.id !== model.id);
  emit('update-saved-models', nextModels);

  if (isSavedModelSelected(model)) {
    try {
      await props.saveConfigAction({
        provider: '',
        apiBase: '',
        apiKey: '',
        model: '',
      });
    } catch (_) {
      return;
    }
  }
};

const currentProviderLabel = computed(() => {
  if (!props.config?.provider) return 'DeepSeek';
  return props.config.provider;
});

const closeSidebar = () => {
  sidebarOpen.value = false;
  isModelDropdownOpen.value = false;
  isHistoryDropdownOpen.value = false;
};

const toggleSidebar = () => {
  sidebarOpen.value = !sidebarOpen.value;
  // Full-viewport z-[90] scrims for model/history menus live in the header; clear them
  // when opening the drawer so they cannot block clicks on the main surface (e.g. settings).
  isModelDropdownOpen.value = false;
  isHistoryDropdownOpen.value = false;
  emit('toggle-sidebar');
};

const onSidebarEscapeKey = (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    closeSidebar();
  }
};

watch(sidebarOpen, (open) => {
  if (open) {
    window.addEventListener('keydown', onSidebarEscapeKey);
  } else {
    window.removeEventListener('keydown', onSidebarEscapeKey);
  }
});

onUnmounted(() => {
  window.removeEventListener('keydown', onSidebarEscapeKey);
});

watch([activeTab, activeMenu], () => {
  isModelDropdownOpen.value = false;
  isHistoryDropdownOpen.value = false;
});

const navigateTo = (section: SidebarSection, settingsView: SettingsSubview = 'dashboard') => {
  if (section === 'chat' || section === 'settings') {
    activeMenu.value = null;
    activeTab.value = section;
    if (section === 'settings') {
      settingsInitialView.value = settingsView;
    }
  } else {
    activeMenu.value = section;
  }

  closeSidebar();
};

const openSettingsFromModelMenu = () => {
  navigateTo('settings', 'providers');
};

const isSectionActive = (section: SidebarSection) => {
  if (section === 'chat' || section === 'settings') {
    return activeMenu.value === null && activeTab.value === section;
  }
  return activeMenu.value === section;
};

const sidebarItemClass = (section: SidebarSection) =>
  isSectionActive(section)
    ? 'bg-pink-50 text-pink-700 border border-pink-100 shadow-sm'
    : 'text-gray-700 hover:bg-gray-100 border border-transparent';

const sidebarIconClass = (section: SidebarSection, activeClass: string) =>
  isSectionActive(section) ? activeClass : 'text-gray-400';

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

const emotionConfig = computed(() => ({
  happy: { emoji: '\u{1F60A}', label: t('emotion.happy') },
  sad: { emoji: '\u{1F622}', label: t('emotion.sad') },
  clingy: { emoji: '\u{1F97A}', label: t('emotion.clingy') },
  jealous: { emoji: '\u{1F624}', label: t('emotion.jealous') },
  angry: { emoji: '\u{1F620}', label: t('emotion.angry') },
  normal: { emoji: '\u{1F642}', label: t('emotion.normal') },
}));

const currentConfig = computed(() => {
  const currentEmotion = props.currentEmotion || 'normal';
  if (currentEmotion in emotionConfig.value) {
    return emotionConfig.value[currentEmotion as keyof typeof emotionConfig.value];
  }
  return emotionConfig.value.normal;
});

const chatBadgeSizeClass = computed(() =>
  props.messages.length < 10 ? 'w-4 h-4 px-0' : 'min-w-[20px] h-4 px-2'
);

const chatBadgeValue = computed(() => {
  const count = props.messages.length;
  if (count > 99) {
    return '99+';
  }
  return String(count);
});

const formatSessionTimestamp = (timestamp: number) => {
  if (!Number.isFinite(timestamp) || timestamp <= 0) {
    return t('chat.unknownTime');
  }
  const date = new Date(timestamp);
  if (Number.isNaN(date.getTime())) {
    return t('chat.unknownTime');
  }
  return date.toLocaleString();
};

defineExpose({
  openSettingsTab(view: SettingsSubview) {
    navigateTo('settings', view);
  },
  openConsole() {
    navigateTo('console');
  },
});
</script>

<template>
  <div class="app-shell w-full h-full flex flex-col overflow-hidden rounded-none relative" :class="`theme-${themeMode}`">
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

    <header class="app-titlebar h-12 flex items-center px-4 relative z-50 border-b drag-region">
      <div class="flex items-center space-x-3">
        <button
          @click="toggleSidebar"
          class="p-1.5 rounded-md transition-colors no-drag"
          :class="sidebarOpen ? 'bg-gray-200 text-gray-800' : 'text-gray-500 hover:bg-gray-200'"
        >
          <Menu :size="18" />
        </button>

        <div class="app-emotion w-8 h-8 rounded-full flex items-center justify-center text-lg shadow-sm border animate-pulse-slow">
          {{ currentConfig.emoji }}
        </div>

        <div class="flex flex-col">
          <h1 class="text-sm font-bold text-gray-800 leading-tight">DiVA</h1>
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
                  'bg-yellow-500 animate-pulse': connectionStatus === 'connecting',
                }"
              />
              <span>
                {{ connectionStatus === 'connected' ? t('app.online') : connectionStatus === 'error' ? t('app.offline') : t('app.connecting') }}
              </span>
            </span>
          </div>
        </div>

        <div class="relative no-drag ml-4 flex space-x-2">
          <button
            v-if="config"
            @click="isModelDropdownOpen = !isModelDropdownOpen"
            class="flex items-center space-x-2 px-2 py-1 bg-gray-50 hover:bg-white border border-gray-200/50 hover:border-pink-200 rounded-lg transition-all text-xs text-gray-600 hover:text-pink-600 shadow-sm group"
            :title="t('app.switchModel')"
          >
            <Server :size="12" class="text-gray-400 group-hover:text-pink-500" />
            <div class="flex flex-col items-start leading-tight">
              <span class="max-w-[100px] truncate font-medium">{{ config.model || t('app.switchModel') }}</span>
              <span class="max-w-[100px] truncate text-[10px] text-gray-400">{{ currentProviderLabel }}</span>
            </div>
          </button>

          <div
            v-if="isModelDropdownOpen"
            class="absolute top-full left-0 mt-1 w-48 bg-white rounded-lg shadow-xl border border-gray-100 overflow-hidden z-[100] animate-in fade-in zoom-in duration-100"
          >
            <div class="py-1 max-h-60 overflow-y-auto">
              <div v-if="savedModels && savedModels.length > 0">
                <div
                  v-for="model in savedModels"
                  :key="model.id"
                  @click="selectSavedModel(model)"
                  class="w-full cursor-pointer px-3 py-2 text-left text-xs hover:bg-pink-50 flex items-center justify-between group"
                  :class="isSavedModelSelected(model) ? 'text-pink-600 font-medium' : 'text-gray-600'"
                >
                  <span class="truncate">{{ model.displayName }}</span>
                  <span class="ml-2 flex items-center gap-2">
                    <Check v-if="isSavedModelSelected(model)" :size="12" class="text-pink-500" />
                    <button
                      type="button"
                      class="rounded p-1 text-gray-400 opacity-0 transition hover:bg-rose-50 hover:text-rose-600 group-hover:opacity-100"
                      :title="t('providers.removeCurrentModel')"
                      @click="removeSavedModel(model, $event)"
                    >
                      <Trash2 :size="12" />
                    </button>
                  </span>
                </div>
              </div>
              <div v-else class="px-3 py-4 text-center text-gray-400 text-[10px]">
                <div class="whitespace-pre-line">{{ t('chat.emptyModels') }}</div>
              </div>

              <div class="border-t border-gray-100 mt-1 pt-1">
                <button
                  @click="openSettingsFromModelMenu"
                  class="w-full text-left px-3 py-2 text-xs text-gray-500 hover:text-gray-800 hover:bg-gray-50 flex items-center"
                >
                  <Settings :size="12" class="mr-2" />
                  {{ t('chat.manageModels') }}
                </button>
              </div>
            </div>
          </div>

          <div v-if="isModelDropdownOpen" class="fixed inset-0 z-[90]" @click="isModelDropdownOpen = false"></div>

          <div class="relative no-drag">
            <button
              @click="isHistoryDropdownOpen = !isHistoryDropdownOpen"
              class="flex items-center space-x-2 px-2 py-1 bg-gray-50 hover:bg-white border border-gray-200/50 hover:border-pink-200 rounded-lg transition-all text-xs text-gray-600 hover:text-pink-600 shadow-sm group"
              :title="t('chat.historySessions')"
            >
              <History :size="14" class="text-gray-400 group-hover:text-pink-500" />
            </button>

            <div
              v-if="isHistoryDropdownOpen"
              class="absolute top-full left-0 mt-1 w-64 bg-white rounded-lg shadow-xl border border-gray-100 overflow-hidden z-[100] animate-in fade-in zoom-in duration-100"
            >
              <div class="py-1 max-h-80 overflow-y-auto">
                <div v-if="sessions && sessions.length > 0">
                  <div
                    v-for="session in sessions"
                    :key="session.chat_id"
                    role="button"
                    tabindex="0"
                    class="w-full text-left px-3 py-2 text-xs hover:bg-pink-50 flex items-center justify-between group text-gray-600 border-b border-gray-50 last:border-0 cursor-pointer"
                    @click="handleSessionSelect(session.session_key)"
                    @keydown.enter="handleSessionSelect(session.session_key)"
                  >
                    <div class="flex flex-col min-w-0 flex-1 pr-2">
                      <span class="text-gray-400 text-[10px] mb-0.5">{{ formatSessionTimestamp(session.timestamp) }}</span>
                      <span class="truncate block w-full text-gray-700">{{ session.snippet || '...' }}</span>
                    </div>
                    <button
                      type="button"
                      class="p-1 rounded text-gray-400 hover:text-rose-500 hover:bg-rose-50 opacity-0 group-hover:opacity-100 transition-opacity shrink-0"
                      :title="t('chat.deleteSession')"
                      @click.stop="handleDeleteSession(session.session_key)"
                    >
                      <Trash2 :size="14" />
                    </button>
                  </div>
                </div>
                <div v-else class="px-3 py-4 text-center text-gray-400 text-xs">
                  <div>{{ t('chat.noHistory') }}</div>
                </div>
              </div>
            </div>
            <div v-if="isHistoryDropdownOpen" class="fixed inset-0 z-[90]" @click="isHistoryDropdownOpen = false"></div>
          </div>
        </div>
      </div>
    </header>

    <!-- z-[60] above titlebar (z-50): full-screen dim; backdrop closes on click (Escape also). -->
    <div v-if="sidebarOpen" class="fixed inset-0 z-[60] pointer-events-none no-drag">
      <div
        class="absolute inset-0 z-0 bg-black/30 backdrop-blur-sm transition-opacity pointer-events-auto"
        aria-hidden="true"
        role="presentation"
        @click="closeSidebar"
      />

      <aside
        class="absolute inset-y-0 left-0 z-10 w-60 bg-white/95 border-r border-gray-200 shadow-xl flex flex-col py-4 px-3 space-y-3 pointer-events-auto"
      >
        <div class="flex items-center px-2 pb-1">
          <div class="w-8 h-8 rounded-xl bg-pink-500 text-white flex items-center justify-center text-lg font-bold shadow-md mr-2">
            V
          </div>
          <div class="flex flex-col">
            <span class="text-sm font-semibold text-gray-800 leading-tight">DiVA</span>
            <span class="text-[10px] text-gray-400 leading-tight">Project ViVY</span>
          </div>
        </div>

        <div class="px-2 pt-1 pb-2 text-xs font-semibold text-gray-500 uppercase tracking-wide">
          {{ t('nav.section') }}
        </div>

        <button
          class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium flex items-center transition-all"
          :class="sidebarItemClass('chat')"
          @click="navigateTo('chat')"
        >
          <span class="flex items-center space-x-2 min-w-0 w-full">
            <MessageSquare :size="16" :class="sidebarIconClass('chat', 'text-pink-500')" />
            <span>{{ t('nav.chat') }}</span>
            <span
              v-if="messages.length > 0"
              :class="[
                'ml-auto bg-red-500 text-white text-[10px] rounded-full flex items-center justify-center leading-none',
                chatBadgeSizeClass,
              ]"
            >
              {{ chatBadgeValue }}
            </span>
          </span>
        </button>

        <button
          class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium flex items-center transition-all"
          :class="sidebarItemClass('settings')"
          @click="navigateTo('settings')"
        >
          <span class="flex items-center space-x-2">
            <Settings :size="16" :class="sidebarIconClass('settings', 'text-emerald-500')" />
            <span>{{ t('nav.settings') }}</span>
          </span>
        </button>

        <button
          class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium flex items-center transition-all"
          :class="sidebarItemClass('console')"
          @click="navigateTo('console')"
        >
          <span class="flex items-center space-x-2">
            <Server :size="16" :class="sidebarIconClass('console', 'text-indigo-500')" />
            <span>{{ t('nav.console') }}</span>
          </span>
        </button>

        <button
          class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium flex items-center transition-all"
          :class="sidebarItemClass('neuro')"
          @click="navigateTo('neuro')"
        >
          <span class="flex items-center space-x-2">
            <Heart :size="16" :class="sidebarIconClass('neuro', 'text-rose-500')" />
            <span>{{ t('nav.neuro') }}</span>
          </span>
        </button>

        <button
          class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium flex items-center transition-all"
          :class="sidebarItemClass('cron')"
          @click="navigateTo('cron')"
        >
          <span class="flex items-center space-x-2">
            <AlarmClock :size="16" :class="sidebarIconClass('cron', 'text-emerald-500')" />
            <span>{{ t('cron.title') }}</span>
          </span>
        </button>
      </aside>
    </div>

    <main
      class="flex-1 min-h-0 overflow-hidden relative z-10 transition-all duration-200"
      :class="sidebarOpen ? 'filter blur-sm scale-[0.99]' : ''"
    >
      <div v-if="activeMenu === 'cron'" class="h-full">
        <CronTaskManagementView />
      </div>
      <div v-else-if="activeMenu" class="h-full flex items-center justify-center">
        <!-- 这个是作者要求不要修改，未经允许禁止往这里面添加东西（未来这里面要放swarm系统的可视化） -->
        <div class="text-gray-500 text-lg font-semibold tracking-wide">
          {{ t('nav.comingSoon') }}
        </div>
      </div>

      <template v-else>
        <div v-if="activeTab === 'chat'" class="h-full">
          <ChatView
            :messages="messages"
            :is-typing="isTyping"
            :theme-mode="themeMode"
            :history-prefs="chatDisplayPrefs"
            @send="(content, attachments) => emit('send', content, attachments)"
            @clear="emit('clear')"
            @stop="emit('stop')"
          />
        </div>
        <div v-else class="h-full min-h-0">
          <SettingsView
            v-if="config && toolsConfig"
            :config="config"
            :provider-configs="providerConfigs"
            :tools-config="toolsConfig"
            :saved-models="savedModels"
            :chat-display-prefs="chatDisplayPrefs"
            :initial-view="settingsInitialView"
            :save-config-action="saveConfigAction"
            :save-tools-config-action="saveToolsConfigAction"
            :save-channel-config-action="saveChannelConfigAction"
            @update-saved-models="handleUpdateSavedModels"
            @save-chat-display-prefs="(prefs) => emit('save-chat-display-prefs', prefs)"
          />
          <div v-else class="h-full flex items-center justify-center text-gray-500">
            Loading configuration...
          </div>
        </div>
      </template>
    </main>

    <AppDialogLayer :theme-mode="themeMode" />
    <AppToastLayer />
  </div>
</template>

<style scoped>
/* Scoped styles */
</style>
