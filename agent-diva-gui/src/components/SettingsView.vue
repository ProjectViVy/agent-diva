<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { ChevronLeft } from 'lucide-vue-next';
import SettingsDashboard from './settings/SettingsDashboard.vue';
import GeneralSettings from './settings/GeneralSettings.vue';
import McpSettings from './settings/McpSettings.vue';
import SkillsSettings from './settings/SkillsSettings.vue';
import ProvidersSettings from './settings/ProvidersSettings.vue';
import ChannelsSettings from './settings/ChannelsSettings.vue';
import NetworkSettings from './settings/NetworkSettings.vue';
import LanguageSettings from './settings/LanguageSettings.vue';
import AboutSettings from './settings/AboutSettings.vue';
import AdvancedSettings from './settings/AdvancedSettings.vue';
import { useI18n } from 'vue-i18n';

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
  | 'advanced'
  | 'about';

const props = defineProps<{
  config: AppConfigShape;
  providerConfigs?: Record<string, ProviderConfigEntry>;
  toolsConfig: ToolsConfigShape;
  savedModels?: SavedModel[];
  chatDisplayPrefs: ChatDisplayPrefs;
  initialView?: SettingsSubview;
  saveConfigAction: (config: AppConfigShape) => Promise<void>;
  saveToolsConfigAction: (tools: ToolsConfigShape) => Promise<void>;
  saveChannelConfigAction: (channelName: string, channelConfig: Record<string, unknown>) => Promise<void>;
}>();

const emit = defineEmits<{
  (e: 'update-saved-models', models: SavedModel[]): void;
  (e: 'save-chat-display-prefs', prefs: ChatDisplayPrefs): void;
}>();

const currentView = ref<SettingsSubview>(props.initialView || 'dashboard');

const pageTitle = computed(() => {
  if (currentView.value === 'dashboard') return t('settings.title');
  const titles = {
    general: t('settings.general'),
    mcp: t('settings.mcp'),
    skills: t('settings.skills'),
    providers: t('settings.providers'),
    channels: t('settings.channels'),
    network: t('settings.network'),
    language: t('settings.language'),
    advanced: t('settings.advanced'),
    about: t('settings.about')
  };
  return titles[currentView.value] || t('settings.title');
});

const handleNavigate = (view: Exclude<SettingsSubview, 'dashboard'>) => {
  currentView.value = view;
};

const goBack = () => {
  currentView.value = 'dashboard';
};

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
  <div class="h-full min-h-0 flex flex-col bg-white rounded-xl overflow-hidden min-w-[320px]">
    <!-- Top Bar -->
    <div class="p-6 border-b border-gray-100 flex justify-between items-center h-20">
      <div class="flex items-center space-x-2">
        <button 
          v-if="currentView !== 'dashboard'"
          @click="goBack"
          class="p-2 hover:bg-gray-100 rounded-lg text-gray-500 transition-colors"
        >
          <ChevronLeft :size="24" />
        </button>
        <h2 class="text-xl font-bold text-gray-800 flex items-center animate-in fade-in slide-in-from-left-2 duration-200" :key="pageTitle">
          <span v-if="currentView === 'dashboard'" class="mr-2">⚙️</span>
          {{ pageTitle }}
        </h2>
      </div>
    </div>
    
    <!-- Content Area -->
    <div class="flex-1 min-h-0 overflow-hidden relative bg-gray-50/30">
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

            <AdvancedSettings v-else-if="currentView === 'advanced'" />
            
            <AboutSettings 
              v-else-if="currentView === 'about'"
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
