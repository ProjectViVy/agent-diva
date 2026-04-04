<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { ChevronLeft, SlidersHorizontal, Bot, WandSparkles, Server, MessageSquare, Search, Globe, Info, Palette } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import SettingsDashboard from './settings/SettingsDashboard.vue';
import GeneralSettings from './settings/GeneralSettings.vue';
import McpSettings from './settings/McpSettings.vue';
import SkillsSettings from './settings/SkillsSettings.vue';
import ProvidersSettings from './settings/ProvidersSettings.vue';
import ChannelsSettings from './settings/ChannelsSettings.vue';
import NetworkSettings from './settings/NetworkSettings.vue';
import LanguageSettings from './settings/LanguageSettings.vue';
import AboutSettings from './settings/AboutSettings.vue';
import ThemeSettings from './settings/ThemeSettings.vue';

const { t } = useI18n();

interface SavedModel {
  id: string;
  provider: string;
  model: string;
  apiBase: string;
  apiKey: string;
  displayName: string;
}
interface ChatDisplayPrefs {
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
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

type SettingsSubview =
  | 'dashboard'
  | 'general'
  | 'mcp'
  | 'skills'
  | 'providers'
  | 'channels'
  | 'network'
  | 'language'
  | 'theme'
  | 'about';

const sectionConfig = computed(() => ({
  general: { icon: SlidersHorizontal, title: t('dashboard.general') },
  mcp: { icon: Bot, title: t('dashboard.mcp') },
  skills: { icon: WandSparkles, title: t('dashboard.skills') },
  providers: { icon: Server, title: t('dashboard.providers') },
  channels: { icon: MessageSquare, title: t('dashboard.channels') },
  network: { icon: Search, title: t('dashboard.network') },
  language: { icon: Globe, title: t('dashboard.language') },
  about: { icon: Info, title: t('dashboard.about') },
  theme: { icon: Palette, title: t('dashboard.theme') },
}));

const props = defineProps<{
  config: AppConfigShape;
  providerConfigs?: Record<string, ProviderConfigEntry>;
  toolsConfig: ToolsConfigShape;
  savedModels?: SavedModel[];
  chatDisplayPrefs: ChatDisplayPrefs;
  themeMode?: string;
  initialView?: SettingsSubview;
  saveConfigAction: (config: AppConfigShape) => Promise<void>;
  saveToolsConfigAction: (tools: ToolsConfigShape) => Promise<void>;
  saveChannelConfigAction: (channelName: string, channelConfig: Record<string, unknown>) => Promise<void>;
}>();

const emit = defineEmits<{
  (e: 'update-saved-models', models: SavedModel[]): void;
  (e: 'save-chat-display-prefs', prefs: ChatDisplayPrefs): void;
  (e: 'change-theme', theme: string): void;
}>();

const currentView = ref<SettingsSubview>(props.initialView || 'dashboard');

const handleNavigate = (view: Exclude<SettingsSubview, 'dashboard'>) => {
  currentView.value = view;
};

const goBack = () => {
  currentView.value = 'dashboard';
};

const currentSection = computed(() => {
  if (currentView.value === 'dashboard') return null;
  return sectionConfig.value[currentView.value as keyof typeof sectionConfig.value];
});

watch(
  () => props.initialView,
  (newView) => {
    if (newView) {
      currentView.value = newView;
    }
  }
);
</script>

<template>
  <div class="settings-container h-full min-h-0 flex flex-col overflow-hidden">
    <!-- Top Navigation Bar (only visible when not on dashboard) -->
    <div
      v-if="currentView !== 'dashboard' && currentSection"
      class="settings-navbar px-4 py-3 flex items-center gap-3"
    >
      <button
        @click="goBack"
        class="settings-navbar-btn"
      >
        <ChevronLeft :size="20" />
      </button>
      <div class="settings-dashboard-icon !mb-0 !w-8 !h-8">
        <component :is="currentSection.icon" :size="18" />
      </div>
      <h2 class="settings-dashboard-title !text-base !mb-0">
        {{ currentSection.title }}
      </h2>
    </div>

    <!-- Content Area -->
    <div class="flex-1 min-h-0 overflow-hidden settings-content">
       <Transition name="page" mode="out-in">
          <div :key="currentView" class="h-full min-h-0 w-full overflow-y-auto">
            <SettingsDashboard
              v-if="currentView === 'dashboard'"
              @navigate="handleNavigate"
            />

            <GeneralSettings
              v-else-if="currentView === 'general'"
              :chat-display-prefs="chatDisplayPrefs"
              @save-chat-display-prefs="(prefs) => emit('save-chat-display-prefs', prefs)"
            />

            <McpSettings
              v-else-if="currentView === 'mcp'"
            />

            <SkillsSettings
              v-else-if="currentView === 'skills'"
            />

            <ProvidersSettings
              v-else-if="currentView === 'providers'"
              :config="config"
              :provider-configs="providerConfigs"
              :saved-models="savedModels"
              :save-config-action="saveConfigAction"
              @update-saved-models="(m) => emit('update-saved-models', m)"
            />

            <ChannelsSettings
              v-else-if="currentView === 'channels'"
              :save-channel-config-action="saveChannelConfigAction"
            />

            <NetworkSettings
              v-else-if="currentView === 'network'"
              :tools-config="toolsConfig"
              :save-tools-config-action="saveToolsConfigAction"
            />

            <LanguageSettings
              v-else-if="currentView === 'language'"
            />

            <AboutSettings
              v-else-if="currentView === 'about'"
            />

            <ThemeSettings
              v-else-if="currentView === 'theme'"
              :current-theme="themeMode || 'love'"
              @change-theme="(theme) => emit('change-theme', theme)"
            />
          </div>
       </Transition>
    </div>
  </div>
</template>

<style scoped>
.page-enter-active,
.page-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}

.page-enter-from {
  opacity: 0;
  transform: translateY(10px);
}

.page-leave-to {
  opacity: 0;
  transform: translateY(-10px);
}
</style>
