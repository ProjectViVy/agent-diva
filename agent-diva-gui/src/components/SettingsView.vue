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
import PetSettings from './settings/PetSettings.vue';
import AboutSettings from './settings/AboutSettings.vue';
import ThemeSettings from './settings/ThemeSettings.vue'
import SelfEvolutionSettings from './settings/SelfEvolutionSettings.vue'
import SandboxSettingsSection from './settings/SandboxSettingsSection.vue'
import CompactionSettings from './settings/CompactionSettings.vue'
import AuditPage from './settings/audit/AuditPage.vue';
import MaskSelectorPanel from './MaskSelectorPanel.vue';
import MaskEditor from './MaskEditor.vue';
import { useMasks } from '../composables/useMasks';
import { useI18n } from 'vue-i18n';
import type { ToolsConfigShape } from '../types/toolsConfig';
import type { MaskEntryDto, MaskPayload } from '../api/desktop';

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
  | 'pet'
  | 'about'
  | 'theme'
  | 'self-evolution'
  | 'sandbox'
  | 'compaction'
  | 'audit'
  | 'masks';

const props = defineProps<{
  config: AppConfigShape;
  providerConfigs?: Record<string, ProviderConfigEntry>;
  toolsConfig: ToolsConfigShape;
  currentSessionKey?: string;
  currentMessages: SettingsMessage[];
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
    pet: t('settings.pet'),
    about: t('settings.about'),
    theme: t('dashboard.theme'),
    'self-evolution': t('dashboard.selfEvolution'),
    sandbox: t('dashboard.sandbox'),
    compaction: t('dashboard.compaction'),
    audit: t('dashboard.audit'),
    masks: '🎭 Masks'
  };
  return titles[currentView.value] || t('settings.title');
});

const handleNavigate = (view: Exclude<SettingsSubview, 'dashboard'>) => {
  currentView.value = view;
};

const goBack = () => {
  currentView.value = 'dashboard';
};

// ---------------------------------------------------------------------------
// Mask editor state
// ---------------------------------------------------------------------------

const showMaskEditor = ref(false);
const editingMaskData = ref<MaskEntryDto | null>(null);
const { create: createOrUpdateMask } = useMasks();

function handleEditMask(mask: MaskEntryDto) {
  editingMaskData.value = mask;
  showMaskEditor.value = true;
}

function handleCreateMask() {
  editingMaskData.value = null;
  showMaskEditor.value = true;
}

async function handleSaveMask(payload: MaskPayload) {
  await createOrUpdateMask(payload);
  showMaskEditor.value = false;
  editingMaskData.value = null;
}

function closeMaskEditor() {
  showMaskEditor.value = false;
  editingMaskData.value = null;
}

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
              :tools-config="toolsConfig"
              :save-tools-config-action="saveToolsConfigAction"
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

            <PetSettings
              v-else-if="currentView === 'pet'"
            />
            
            <AboutSettings
              v-else-if="currentView === 'about'"
            />

            <div v-else-if="currentView === 'theme'">
              <ThemeSettings :current-theme="themeMode || 'love'" @change-theme="emit('change-theme', $event)" />
            </div>
            <div v-else-if="currentView === 'self-evolution'">
              <SelfEvolutionSettings />
            </div>
            <div v-else-if="currentView === 'sandbox'">
              <SandboxSettingsSection />
            </div>
            <div v-else-if="currentView === 'compaction'">
              <CompactionSettings
                :tools-config="toolsConfig"
                :current-session-key="currentSessionKey"
                :current-messages="currentMessages"
                :save-tools-config-action="saveToolsConfigAction"
              />
            </div>
            <div v-else-if="currentView === 'audit'" class="h-full min-h-0 overflow-y-auto">
              <AuditPage />
            </div>
            <div v-else-if="currentView === 'masks'" class="h-full min-h-0 overflow-y-auto p-6">
              <MaskSelectorPanel
                mode="manager"
                @edit="handleEditMask"
                @create="handleCreateMask"
              />
            </div>
          </div>
       </Transition>
    </div>

    <!-- MaskEditor modal overlay -->
    <Teleport to="body">
      <div
        v-if="showMaskEditor"
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
        @click.self="closeMaskEditor"
      >
        <MaskEditor
          :mask="editingMaskData ?? undefined"
          @save="handleSaveMask"
          @cancel="closeMaskEditor"
        />
      </div>
    </Teleport>
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
