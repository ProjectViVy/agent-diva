<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { Sparkles, LoaderCircle, BrainCircuit, ShieldQuestion } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { showAppToast } from '../../utils/appToast';


const { t } = useI18n();

interface SelfEvolutionConfig {
  enabled: boolean;
  autodream_frequency: 'daily' | 'weekly' | 'manual';
  trigger_threshold_sessions: number;
  trigger_threshold_messages: number;
  auto_merge_confidence: number;
  require_confirmation_for: string[];
}

const FREQUENCY_OPTIONS: SelfEvolutionConfig['autodream_frequency'][] = ['daily', 'weekly', 'manual'];
const CONFIRM_ACTION_OPTIONS = ['identity', 'relationship', 'commitment', 'sop', 'deprecation'];

const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);
const config = ref<SelfEvolutionConfig>({
  enabled: false,
  autodream_frequency: 'weekly',
  trigger_threshold_sessions: 10,
  trigger_threshold_messages: 50,
  auto_merge_confidence: 0.95,
  require_confirmation_for: [],
});
const originalSnapshot = ref('');

const isDirty = computed(() => JSON.stringify(config.value) !== originalSnapshot.value);

const autoMergeConfidencePercent = computed(() => Math.round(config.value.auto_merge_confidence * 100));

const loadConfig = async () => {
  loading.value = true;
  loadError.value = null;
  try {
    const data = await invoke<SelfEvolutionConfig>('get_self_evolution_config');
    config.value = { ...data };
    originalSnapshot.value = JSON.stringify(data);
  } catch {
    // Backend command may not exist yet; use defaults so UI is still usable
    config.value = {
      enabled: false,
      autodream_frequency: 'weekly',
      trigger_threshold_sessions: 10,
      trigger_threshold_messages: 50,
      auto_merge_confidence: 0.95,
      require_confirmation_for: [],
    };
    originalSnapshot.value = JSON.stringify(config.value);
  } finally {
    loading.value = false;
  }
};

const saveConfig = async () => {
  if (saving.value || !isDirty.value) return;
  saving.value = true;
  try {
    await invoke('save_self_evolution_config', { config: { ...config.value } });
    originalSnapshot.value = JSON.stringify(config.value);
    showAppToast(t('selfEvolution.saved'), 'success');
  } catch (e) {
    showAppToast(t('selfEvolution.saveFailed'), 'error');
  } finally {
    saving.value = false;
  }
};

const toggleConfirmAction = (action: string) => {
  const idx = config.value.require_confirmation_for.indexOf(action);
  if (idx >= 0) {
    config.value.require_confirmation_for.splice(idx, 1);
  } else {
    config.value.require_confirmation_for.push(action);
  }
};

onMounted(loadConfig);
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <!-- Header -->
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center space-x-3">
        <div class="settings-dashboard-icon">
          <Sparkles :size="20" />
        </div>
        <div>
          <h3 class="settings-dashboard-title">{{ t('selfEvolution.title') }}</h3>
          <p class="settings-dashboard-desc">{{ t('selfEvolution.desc') }}</p>
        </div>
      </div>
      <button
        type="button"
        class="settings-btn settings-btn-primary inline-flex min-w-[112px] items-center justify-center gap-2"
        :disabled="saving || !isDirty"
        @click="saveConfig"
      >
        <LoaderCircle v-if="saving" :size="16" class="animate-spin" />
        <span>{{ saving ? t('selfEvolution.saving') : t('selfEvolution.save') }}</span>
      </button>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="flex items-center justify-center py-12">
      <LoaderCircle :size="24" class="animate-spin" :style="{ color: 'var(--accent)' }" />
      <span class="ml-3 settings-muted">{{ t('selfEvolution.loading') }}</span>
    </div>

    <!-- Error -->
    <div v-else-if="loadError" class="settings-section" :style="{ borderColor: 'var(--danger)' }">
      <p class="text-sm" :style="{ color: 'var(--danger)' }">{{ t('selfEvolution.loadError') }}: {{ loadError }}</p>
      <button type="button" class="settings-btn settings-btn-secondary mt-3" @click="loadConfig">
        {{ t('selfEvolution.retry') }}
      </button>
    </div>

    <!-- Content -->
    <template v-else>
      <!-- Group 1: AutoDream Settings -->
      <div class="settings-section space-y-5">
        <div class="settings-section-header">
          <BrainCircuit :size="16" />
          <span>{{ t('selfEvolution.autoDreamGroup') }}</span>
        </div>

        <!-- Master Toggle -->
        <label class="settings-label flex items-center justify-between cursor-pointer">
          <span>{{ t('selfEvolution.enabled') }}</span>
          <button
            type="button"
            role="switch"
            :aria-checked="config.enabled"
            class="self-evo-toggle"
            :class="{ active: config.enabled }"
            @click="config.enabled = !config.enabled"
          >
            <span class="self-evo-toggle-thumb" />
          </button>
        </label>

        <!-- Frequency Dropdown -->
        <div class="space-y-1">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">
            {{ t('selfEvolution.autodreamFrequency') }}
          </label>
          <select v-model="config.autodream_frequency" class="settings-input">
            <option v-for="opt in FREQUENCY_OPTIONS" :key="opt" :value="opt">
              {{ t(`selfEvolution.frequency.${opt}`) }}
            </option>
          </select>
        </div>

        <!-- Trigger Threshold: Sessions -->
        <div class="space-y-1">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">
            {{ t('selfEvolution.triggerThresholdSessions') }}
          </label>
          <input
            v-model.number="config.trigger_threshold_sessions"
            type="number"
            min="1"
            step="1"
            class="settings-input"
          />
          <p class="text-xs settings-muted">{{ t('selfEvolution.triggerThresholdSessionsHint') }}</p>
        </div>

        <!-- Trigger Threshold: Messages -->
        <div class="space-y-1">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">
            {{ t('selfEvolution.triggerThresholdMessages') }}
          </label>
          <input
            v-model.number="config.trigger_threshold_messages"
            type="number"
            min="1"
            step="1"
            class="settings-input"
          />
          <p class="text-xs settings-muted">{{ t('selfEvolution.triggerThresholdMessagesHint') }}</p>
        </div>
      </div>

      <!-- Group 2: Trust & Safety -->
      <div class="settings-section space-y-5">
        <div class="settings-section-header">
          <ShieldQuestion :size="16" />
          <span>{{ t('selfEvolution.trustGroup') }}</span>
        </div>

        <!-- Auto-Merge Confidence Slider -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <label class="text-xs font-medium settings-muted uppercase tracking-wider">
              {{ t('selfEvolution.autoMergeConfidence') }}
            </label>
            <span class="text-xs font-mono" :style="{ color: 'var(--accent)' }">{{ autoMergeConfidencePercent }}%</span>
          </div>
          <input
            v-model.number="config.auto_merge_confidence"
            type="range"
            min="0"
            max="1"
            step="0.01"
            class="self-evo-slider"
          />
          <p class="text-xs settings-muted">{{ t('selfEvolution.autoMergeConfidenceHint') }}</p>
        </div>

        <!-- Require Confirmation For Checkboxes -->
        <div class="space-y-2">
          <label class="block text-xs font-medium settings-muted uppercase tracking-wider">
            {{ t('selfEvolution.requireConfirmationFor') }}
          </label>
          <p class="text-xs settings-muted mb-2">{{ t('selfEvolution.requireConfirmationForHint') }}</p>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
            <label
              v-for="action in CONFIRM_ACTION_OPTIONS"
              :key="action"
              class="settings-label flex items-center space-x-2 cursor-pointer"
            >
              <input
                type="checkbox"
                class="settings-checkbox"
                :checked="config.require_confirmation_for.includes(action)"
                @change="toggleConfirmAction(action)"
              />
              <span>{{ t(`selfEvolution.confirmActions.${action}`) }}</span>
            </label>
          </div>
        </div>
      </div>
    </template>

  </div>
</template>

<style scoped>
.fade-in {
  animation: slideIn 0.3s ease-out;
}

@keyframes slideIn {
  from { opacity: 0; transform: translateX(20px); }
  to { opacity: 1; transform: translateX(0); }
}

.self-evo-toggle {
  position: relative;
  width: 44px;
  height: 24px;
  border-radius: 12px;
  border: none;
  cursor: pointer;
  transition: background-color 0.2s ease;
  background: var(--line);
  flex-shrink: 0;
}

.self-evo-toggle.active {
  background: var(--accent);
}

.self-evo-toggle-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s ease;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.2);
}

.self-evo-toggle.active .self-evo-toggle-thumb {
  transform: translateX(20px);
}

.self-evo-slider {
  width: 100%;
  height: 6px;
  border-radius: 3px;
  appearance: none;
  -webkit-appearance: none;
  background: var(--line);
  outline: none;
  cursor: pointer;
}

.self-evo-slider::-webkit-slider-thumb {
  appearance: none;
  -webkit-appearance: none;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
}

.self-evo-slider::-moz-range-thumb {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--accent);
  cursor: pointer;
  border: none;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.2);
}
</style>
