<script setup lang="ts">
import { ref, computed, watch } from 'vue';
import { ChevronLeft } from '@lucide/vue';
import SettingsDashboard from './settings/SettingsDashboard.vue';
import GeneralSettings from './settings/GeneralSettings.vue';
import McpSettings from './settings/McpSettings.vue';
import SkillsSettings from './settings/SkillsSettings.vue';
import ProvidersSettings from './settings/ProvidersSettings.vue';
import ChannelsSettings from './settings/ChannelsSettings.vue';
import NetworkSettings from './settings/NetworkSettings.vue';
import LanguageSettings from './settings/LanguageSettings.vue';
import AboutSettings from './settings/AboutSettings.vue';
import ThemeSettings from './settings/ThemeSettings.vue'
import SandboxSettingsSection from './settings/SandboxSettingsSection.vue'
import SpeechSettings from './settings/SpeechSettings.vue'
import PersonaMemoryView from './persona-memory/PersonaMemoryView.vue'
import MemoryView from './MemoryView.vue'
import EvolutionView from './EvolutionView.vue'
import CompactionSettings from './settings/CompactionSettings.vue'
import MasksSettings from './settings/MasksSettings.vue'
import { useI18n } from 'vue-i18n';
import type { ChatDisplayPrefs, SavedModel } from '../types/chat-ui';

const { t } = useI18n();

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

interface SettingsMessage {
  role: 'user' | 'agent' | 'system' | 'tool';
  content: string;
  reasoning?: string;
  rawMeta?: Record<string, unknown>;
  fromHistory?: boolean;
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
  | 'about'
  | 'theme'
  | 'sandbox'
  | 'compaction'
  | 'masks'
  | 'voice'
  | 'persona'
  | 'memory'
  | 'evolution';

const props = defineProps<{
  config: AppConfigShape;
  providerConfigs?: Record<string, ProviderConfigEntry>;
  currentSessionKey?: string;
  currentMessages: SettingsMessage[];
  savedModels?: SavedModel[];
  chatDisplayPrefs: ChatDisplayPrefs;
  themeMode?: string;
  initialView?: SettingsSubview;
  saveConfigAction: (config: AppConfigShape) => Promise<void>;
}>();

const emit = defineEmits<{
  (e: 'update-saved-models', models: SavedModel[]): void;
  (e: 'save-chat-display-prefs', prefs: ChatDisplayPrefs): void;
  (e: 'change-theme', theme: string): void;
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
    about: t('settings.about'),
    theme: t('dashboard.theme'),
    sandbox: t('dashboard.sandbox'),
    compaction: t('dashboard.compaction'),
    masks: t('dashboard.masks'),
    voice: t('dashboard.voice'),
    persona: t('dashboard.persona'),
    memory: t('dashboard.memory'),
    evolution: t('dashboard.evolution'),
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
  <div class="settings-shell">
    <div class="settings-subheader">
      <div class="settings-subheader-inner">
        <button 
          v-if="currentView !== 'dashboard'"
          @click="goBack"
          class="settings-back-btn"
        >
          <ChevronLeft :size="24" />
        </button>
        <h2 class="settings-page-title animate-in fade-in slide-in-from-left-2 duration-200" :key="pageTitle">
          {{ pageTitle }}
        </h2>
      </div>
    </div>
    
    <div class="settings-body">
       <Transition name="page" mode="out-in">
          <div :key="currentView" class="settings-view-panel">
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
            />

            <NetworkSettings
              v-else-if="currentView === 'network'"
            />
            
            <LanguageSettings 
              v-else-if="currentView === 'language'"
            />

            <AboutSettings
              v-else-if="currentView === 'about'"
            />

            <div v-else-if="currentView === 'theme'">
              <ThemeSettings :current-theme="themeMode || 'love'" @change-theme="emit('change-theme', $event)" />
            </div>
            <div v-else-if="currentView === 'sandbox'">
              <SandboxSettingsSection />
            </div>
            <div v-else-if="currentView === 'voice'">
              <SpeechSettings />
            </div>
            <div v-else-if="currentView === 'compaction'">
              <CompactionSettings
                :current-session-key="currentSessionKey"
                :current-messages="currentMessages"
              />
            </div>
            <div v-else-if="currentView === 'masks'">
              <MasksSettings :current-session-key="currentSessionKey" />
            </div>
            <div v-else-if="currentView === 'persona'">
              <PersonaMemoryView />
            </div>
            <div v-else-if="currentView === 'memory'">
              <MemoryView />
            </div>
            <div v-else-if="currentView === 'evolution'">
              <EvolutionView />
            </div>
          </div>
       </Transition>
    </div>

  </div>
</template>

<style scoped>
.settings-shell {
  display: flex;
  flex-direction: column;
  min-width: 320px;
  min-height: 0;
  height: 100%;
}

.settings-subheader {
  padding: 12px 24px 6px;
  background: transparent;
}

.settings-subheader-inner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  min-height: 40px;
}

.settings-back-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: 1px solid transparent;
  border-radius: 999px;
  background: transparent;
  color: var(--text-muted);
  transition: background-color 0.15s ease, color 0.15s ease, border-color 0.15s ease;
}

.settings-back-btn:hover {
  background: var(--nav-hover);
  border-color: var(--line);
  color: var(--text);
}

.settings-page-title {
  margin: 0;
  font-size: 1.125rem;
  line-height: 1.75rem;
  font-weight: 600;
  color: var(--text);
}

.settings-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  position: relative;
}

.settings-view-panel {
  height: 100%;
  min-height: 0;
  width: 100%;
  overflow-y: auto;
}

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
