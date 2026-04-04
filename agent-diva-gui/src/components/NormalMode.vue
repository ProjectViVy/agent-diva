<script setup lang="ts">
import { computed, defineExpose, onMounted, onUnmounted, ref, watch } from 'vue';
import {
  AlarmClock,
  Bot,
  Check,
  ChevronDown,
  Heart,
  Menu,
  MessageSquare,
  Server,
  Settings,
  Trash2,
  WandSparkles,
  Wrench,
  Zap,
} from 'lucide-vue-next';
import ChatView from './ChatView.vue';
import SettingsView from './SettingsView.vue';
import CronTaskManagementView from './CronTaskManagementView.vue';
import ConsoleView from './ConsoleView.vue';
import McpSettings from './settings/McpSettings.vue';
import SkillsSettings from './settings/SkillsSettings.vue';
import AppDialogLayer from './AppDialogLayer.vue';
import AppToastLayer from './AppToastLayer.vue';
import { useI18n } from 'vue-i18n';

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
  (e: 'send', content: string): void;
  (e: 'clear'): void;
  (e: 'stop'): void;
  (e: 'toggle-sidebar'): void;
  (e: 'update-saved-models', models: SavedModel[]): void;
  (e: 'save-chat-display-prefs', prefs: ChatDisplayPrefs): void;
  (e: 'load-session', sessionKey: string): void;
  (e: 'delete-session', sessionKey: string): void;
}>();

type SidebarSection = 'chat' | 'settings' | 'console' | 'neuro' | 'cron' | 'mcp' | 'skills';

const activeTab = ref<'chat' | 'settings'>('chat');
const activeMenu = ref<'console' | 'neuro' | 'cron' | 'mcp' | 'skills' | null>(null);
const settingsInitialView = ref<SettingsSubview>('dashboard');
const sidebarOpen = ref(false);
const sidebarCollapsed = ref(true);
const sidebarAutoCollapsed = ref(false);
const groups = ref({ capabilities: true, tools: true });
const themeMode = ref('love');
const isModelDropdownOpen = ref(false);
const activeSessionKey = ref('');

// 收缩状态下的弹出菜单
const collapsedPopup = ref<{ type: 'capabilities' | 'tools' | null; x: number; y: number }>({
  type: null,
  x: 0,
  y: 0,
});

const handleClearSession = () => {
  activeSessionKey.value = '';
  emit('clear');
};

const handleUpdateSavedModels = (models: SavedModel[]) => {
  emit('update-saved-models', models);
};

const handleChangeTheme = (theme: string) => {
  themeMode.value = theme;
  // 应用主题到 document.documentElement
  document.documentElement.setAttribute('data-theme', theme);
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
};

const toggleSidebar = () => {
  sidebarCollapsed.value = !sidebarCollapsed.value;
  sidebarAutoCollapsed.value = false;
  // Full-viewport z-[90] scrims for model menus live in the header; clear them
  // when opening the drawer so they cannot block clicks on the main surface (e.g. settings).
  isModelDropdownOpen.value = false;
  emit('toggle-sidebar');
};

const toggleGroup = (groupName: keyof typeof groups.value) => {
  groups.value[groupName] = !groups.value[groupName];
};

const handleCollapsedGroupClick = (type: 'capabilities' | 'tools', event: MouseEvent) => {
  if (sidebarCollapsed.value) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    collapsedPopup.value = {
      type: collapsedPopup.value.type === type ? null : type,
      x: rect.right + 8,
      y: rect.top,
    };
  } else {
    toggleGroup(type);
  }
};

const closeCollapsedPopup = () => {
  collapsedPopup.value.type = null;
};

const handleNavigateAndClose = (section: SidebarSection, settingsView: SettingsSubview = 'dashboard') => {
  navigateTo(section, settingsView);
  closeCollapsedPopup();
};

const handleResize = () => {
  sidebarAutoCollapsed.value = window.innerWidth < 768;
  if (sidebarAutoCollapsed.value) {
    sidebarCollapsed.value = true;
  }
};

const onSidebarEscapeKey = (e: KeyboardEvent) => {
  if (e.key === 'Escape') {
    closeSidebar();
  }
};

watch(sidebarCollapsed, (collapsed) => {
  if (!collapsed) {
    window.addEventListener('keydown', onSidebarEscapeKey);
  } else {
    window.removeEventListener('keydown', onSidebarEscapeKey);
  }
});

onMounted(() => {
  handleResize();
  window.addEventListener('resize', handleResize);
});

onUnmounted(() => {
  window.removeEventListener('keydown', onSidebarEscapeKey);
  window.removeEventListener('resize', handleResize);
});

watch([activeTab, activeMenu], () => {
  isModelDropdownOpen.value = false;
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

  if (sidebarAutoCollapsed.value) {
    sidebarCollapsed.value = true;
  }
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

// @ts-ignore - reserved for future use
const _currentConfig = computed(() => {
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

// Reserved for future use
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
  <div class="app-shell" :class="{ 'sidebar-expanded': !sidebarCollapsed, [`theme-${themeMode}`]: true }">
    <!-- Love主题背景装饰 -->
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

    <!-- 常驻侧边栏 -->
    <aside class="sidebar" :class="{ 'sidebar-collapsed': sidebarCollapsed }">
      <!-- Logo区域 -->
      <div class="sidebar-header">
        <div class="brand-logo">V</div>
        <span v-if="!sidebarCollapsed" class="brand-text">DiVA</span>
      </div>

      <!-- Menu按钮（折叠控制） -->
      <button @click="toggleSidebar" class="menu-toggle no-drag">
        <Menu :size="18" />
      </button>

      <!-- 导航区域 -->
      <nav class="sidebar-nav">
        <!-- 主导航项 -->
        <button class="nav-item" :class="{ active: isSectionActive('chat') }" @click="navigateTo('chat')">
          <MessageSquare />
          <span v-if="!sidebarCollapsed">{{ t('nav.chat') }}</span>
          <span
            v-if="!sidebarCollapsed && messages.length > 0"
            class="ml-auto bg-red-500 text-white text-[10px] rounded-full flex items-center justify-center leading-none"
            :class="chatBadgeSizeClass"
          >
            {{ chatBadgeValue }}
          </span>
        </button>
        <button class="nav-item" :class="{ active: isSectionActive('console') }" @click="navigateTo('console')">
          <Server />
          <span v-if="!sidebarCollapsed">{{ t('nav.console') }}</span>
        </button>

        <!-- NavGroup: Capabilities -->
        <div class="nav-group">
          <div class="nav-group-header" @click.stop="handleCollapsedGroupClick('capabilities', $event)">
            <Zap />
            <span v-if="!sidebarCollapsed">{{ t('nav.capabilities') }}</span>
            <div v-if="!sidebarCollapsed" class="nav-group-chevron">
              <ChevronDown v-if="groups.capabilities" />
              <ChevronDown v-else class="rotate-[-90deg]" />
            </div>
          </div>
          <div v-show="!sidebarCollapsed && groups.capabilities" class="nav-group-items">
            <button class="nav-item nav-item-sub" :class="{ active: isSectionActive('neuro') }" @click="handleNavigateAndClose('neuro')">
              <Heart />
              <span>{{ t('nav.neuro') }}</span>
            </button>
            <button class="nav-item nav-item-sub" :class="{ active: isSectionActive('cron') }" @click="handleNavigateAndClose('cron')">
              <AlarmClock />
              <span>{{ t('cron.title') }}</span>
            </button>
          </div>
        </div>

        <!-- NavGroup: Tools -->
        <div class="nav-group">
          <div class="nav-group-header" @click.stop="handleCollapsedGroupClick('tools', $event)">
            <Wrench />
            <span v-if="!sidebarCollapsed">{{ t('nav.toolsGroup') }}</span>
            <div v-if="!sidebarCollapsed" class="nav-group-chevron">
              <ChevronDown v-if="groups.tools" />
              <ChevronDown v-else class="rotate-[-90deg]" />
            </div>
          </div>
          <div v-show="!sidebarCollapsed && groups.tools" class="nav-group-items">
            <button class="nav-item nav-item-sub" :class="{ active: isSectionActive('mcp') }" @click="handleNavigateAndClose('mcp')">
              <Bot />
              <span>{{ t('dashboard.mcp') }}</span>
            </button>
            <button class="nav-item nav-item-sub" :class="{ active: isSectionActive('skills') }" @click="handleNavigateAndClose('skills')">
              <WandSparkles />
              <span>{{ t('dashboard.skills') }}</span>
            </button>
          </div>
        </div>
      </nav>

      <!-- 底部区域 -->
      <div class="sidebar-footer">
        <button class="nav-item" :class="{ active: isSectionActive('settings') }" @click="navigateTo('settings')">
          <Settings />
          <span v-if="!sidebarCollapsed">{{ t('nav.settings') }}</span>
        </button>
      </div>
    </aside>

    <!-- 收缩状态下的弹出菜单 -->
    <div
      v-if="collapsedPopup.type && sidebarCollapsed"
      class="fixed z-[200] min-w-[180px] bg-white rounded-lg shadow-xl border border-gray-100 overflow-hidden"
      :style="{ left: `${collapsedPopup.x}px`, top: `${collapsedPopup.y}px` }"
      @click.stop
    >
      <div class="py-1">
        <!-- Capabilities 菜单 -->
        <template v-if="collapsedPopup.type === 'capabilities'">
          <button
            class="popup-menu-item"
            :class="{ active: isSectionActive('neuro') }"
            @click="handleNavigateAndClose('neuro')"
          >
            <Heart class="popup-menu-icon" />
            <span>{{ t('nav.neuro') }}</span>
          </button>
          <button
            class="popup-menu-item"
            :class="{ active: isSectionActive('cron') }"
            @click="handleNavigateAndClose('cron')"
          >
            <AlarmClock class="popup-menu-icon" />
            <span>{{ t('cron.title') }}</span>
          </button>
        </template>
        <!-- Tools 菜单 -->
        <template v-if="collapsedPopup.type === 'tools'">
          <button
            class="popup-menu-item"
            :class="{ active: isSectionActive('mcp') }"
            @click="handleNavigateAndClose('mcp')"
          >
            <Bot class="popup-menu-icon" />
            <span>{{ t('dashboard.mcp') }}</span>
          </button>
          <button
            class="popup-menu-item"
            :class="{ active: isSectionActive('skills') }"
            @click="handleNavigateAndClose('skills')"
          >
            <WandSparkles class="popup-menu-icon" />
            <span>{{ t('dashboard.skills') }}</span>
          </button>
        </template>
      </div>
    </div>
    <!-- 点击遮罩关闭弹出菜单 -->
    <div v-if="collapsedPopup.type" class="fixed inset-0 z-[190]" @click="closeCollapsedPopup"></div>

    <!-- 主内容区 -->
    <main class="main-panel">
      <!-- Topbar -->
      <header class="topbar drag-region">
        <div class="topbar-left no-drag">
          <!-- DIVA 头像和状态 -->
          <div class="topbar-identity">
            <div class="topbar-avatar">
              {{ emotionConfig[(props.currentEmotion || 'happy') as keyof typeof emotionConfig]?.emoji || '😊' }}
            </div>
            <div class="topbar-identity-info">
              <div class="topbar-identity-name">
                DIVA
                <span class="topbar-emotion-label">{{ emotionConfig[(props.currentEmotion || 'happy') as keyof typeof emotionConfig]?.label || t('emotion.happy') }}</span>
              </div>
              <div class="topbar-status">
                <div
                  class="topbar-status-dot"
                  :class="{
                    'connected': connectionStatus === 'connected',
                    'error': connectionStatus === 'error',
                    'connecting': connectionStatus === 'connecting',
                  }"
                />
                <span>
                  {{ connectionStatus === 'connected' ? t('app.online') : connectionStatus === 'error' ? t('app.offline') : t('app.connecting') }}
                </span>
              </div>
            </div>
          </div>
        </div>

        <div class="topbar-right no-drag">
          <!-- Model下拉 -->
          <div class="relative">
            <button
              v-if="config"
              @click="isModelDropdownOpen = !isModelDropdownOpen"
              class="flex items-center space-x-2 px-2 py-1 bg-gray-50 hover:bg-white border border-gray-200/50 hover:border-pink-200 rounded-lg transition-all text-xs text-gray-600 hover:text-pink-600 shadow-sm group"
            >
              <Server :size="12" class="text-gray-400 group-hover:text-pink-500" />
              <div class="flex flex-col items-start leading-tight">
                <span class="max-w-[100px] truncate font-medium">{{ config.model || t('app.switchModel') }}</span>
                <span class="max-w-[100px] truncate text-[10px] text-gray-400">{{ currentProviderLabel }}</span>
              </div>
            </button>

            <!-- Model下拉菜单内容 -->
            <div v-if="isModelDropdownOpen" class="absolute top-full right-0 mt-1 w-48 bg-white rounded-lg shadow-xl border border-gray-100 overflow-hidden z-[100]">
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
          </div>

          
        </div>
      </header>

      <!-- 内容区域 -->
      <div class="content-area">
        <!-- Console视图 -->
        <div v-if="activeMenu === 'console'" class="h-full">
          <ConsoleView />
        </div>
        <!-- Cron视图 -->
        <div v-else-if="activeMenu === 'cron'" class="h-full">
          <CronTaskManagementView />
        </div>
        <!-- MCP视图 -->
        <div v-else-if="activeMenu === 'mcp'" class="h-full">
          <div class="h-full min-h-0 flex flex-col subview-container">
            <div class="flex-1 min-h-0 overflow-hidden">
              <div class="h-full min-h-0 w-full overflow-y-auto p-6">
                <McpSettings />
              </div>
            </div>
          </div>
        </div>
        <!-- Skills视图 -->
        <div v-else-if="activeMenu === 'skills'" class="h-full">
          <div class="h-full min-h-0 flex flex-col subview-container">
            <div class="flex-1 min-h-0 overflow-hidden">
              <div class="h-full min-h-0 w-full overflow-y-auto p-6">
                <SkillsSettings />
              </div>
            </div>
          </div>
        </div>
        <!-- 占位视图（neuro等） -->
        <div v-else-if="activeMenu" class="h-full flex items-center justify-center">
          <!-- 这个是作者要求不要修改，未经允许禁止往这里面添加东西（未来这里面要放swarm系统的可视化） -->
          <div class="text-gray-500 text-lg font-semibold tracking-wide">
            {{ t('nav.comingSoon') }}
          </div>
        </div>

        <!-- 聊天/设置视图 -->
        <template v-else>
          <div v-if="activeTab === 'chat'" class="h-full">
            <ChatView
              :messages="messages"
              :is-typing="isTyping"
              :theme-mode="themeMode"
              :history-prefs="chatDisplayPrefs"
              :sessions="sessions"
              :active-session-key="activeSessionKey"
              @send="(content) => emit('send', content)"
              @clear="handleClearSession"
              @stop="emit('stop')"
              @select-session="(key) => emit('load-session', key)"
              @delete-session="(key) => emit('delete-session', key)"
              @new-session="handleClearSession"
              @toggle-pin="(_key) => {}"
              @rename-session="(_key, _title) => {}"
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
              :theme-mode="themeMode"
              :initial-view="settingsInitialView"
              :save-config-action="saveConfigAction"
              :save-tools-config-action="saveToolsConfigAction"
              :save-channel-config-action="saveChannelConfigAction"
              @update-saved-models="handleUpdateSavedModels"
              @save-chat-display-prefs="(prefs) => emit('save-chat-display-prefs', prefs)"
              @change-theme="handleChangeTheme"
            />
            <div v-else class="h-full flex items-center justify-center text-gray-500">
              Loading configuration...
            </div>
          </div>
        </template>
      </div>
    </main>

    <AppDialogLayer :theme-mode="themeMode" />
    <AppToastLayer />
  </div>
</template>

<style scoped>
/* Scoped styles */
</style>
