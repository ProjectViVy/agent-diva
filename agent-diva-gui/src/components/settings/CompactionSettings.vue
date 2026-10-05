<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { Minimize2, LoaderCircle, RotateCcw } from '@lucide/vue';
import { useI18n } from 'vue-i18n';
import { vivyClient } from '../../api/vivy/instance';
import {
  budgetShapeFromCompaction,
  loadCompactionConfig,
  saveCompactionConfig,
  type CompactionConfigShape,
} from '../../api/settings';
import { showAppToast } from '../../utils/appToast';
import { budgetPressurePercent, computeBudgetStatus } from '../../utils/contextBudget';

const { t } = useI18n();

interface SettingsMessage {
  role: 'user' | 'agent' | 'system' | 'tool';
  content: string;
  reasoning?: string;
  rawMeta?: Record<string, unknown>;
  fromHistory?: boolean;
}

const props = defineProps<{
  currentSessionKey?: string;
  currentMessages: SettingsMessage[];
}>();

const config = ref<CompactionConfigShape | null>(null);
const draft = ref({ enabled: true, maxTokens: 0, triggerPercent: 80, keepRecent: 12 });
const loading = ref(false);
const isSaving = ref(false);
const compactRunning = ref(false);
const loadError = ref('');

const budgetStatus = computed(() =>
  config.value
    ? computeBudgetStatus(props.currentMessages, budgetShapeFromCompaction(config.value))
    : null
);

const isDirty = computed(() => {
  if (!config.value) return false;
  return (
    draft.value.enabled !== config.value.enabled ||
    draft.value.maxTokens !== config.value.maxTokens ||
    draft.value.triggerPercent !== config.value.triggerPercent ||
    draft.value.keepRecent !== config.value.keepRecent
  );
});

const pressurePercent = computed(() =>
  budgetStatus.value ? budgetPressurePercent(budgetStatus.value) : 0
);

const pressureColor = computed(() => {
  const p = pressurePercent.value;
  if (p < 60) return 'bg-success';
  if (p < 80) return 'bg-warning';
  return 'bg-destructive';
});

async function refresh() {
  loading.value = true;
  loadError.value = '';
  try {
    config.value = await loadCompactionConfig();
    draft.value = {
      enabled: config.value.enabled,
      maxTokens: config.value.maxTokens,
      triggerPercent: config.value.triggerPercent,
      keepRecent: config.value.keepRecent,
    };
  } catch (err) {
    loadError.value = String(err);
  } finally {
    loading.value = false;
  }
}

const saveConfig = async () => {
  if (isSaving.value || !isDirty.value) return;
  isSaving.value = true;
  try {
    await saveCompactionConfig({
      enabled: draft.value.enabled,
      maxTokens: Math.min(500000, Math.max(10000, Number(draft.value.maxTokens) || 0)),
      triggerPercent: Math.min(100, Math.max(10, Number(draft.value.triggerPercent) || 80)),
      keepRecent: Math.min(50, Math.max(1, Number(draft.value.keepRecent) || 12)),
    });
    showAppToast(t('settings.saved'), 'success');
    await refresh();
  } catch (err) {
    loadError.value = String(err);
  } finally {
    isSaving.value = false;
  }
};

const resetDefaults = async () => {
  if (!config.value) return;
  draft.value = {
    enabled: config.value.configEnabled,
    maxTokens: config.value.configMaxTokens,
    triggerPercent: config.value.configTriggerPercent,
    keepRecent: config.value.configKeepRecent,
  };
  await saveConfig();
};

const runCompact = async () => {
  compactRunning.value = true;
  try {
    const sessionKey = props.currentSessionKey?.trim() || '';
    if (!sessionKey) throw new Error('no active session');
    await vivyClient.contextCompact(sessionKey);
    showAppToast(t('compaction.compactSuccess'), 'success');
  } catch {
    showAppToast(t('compaction.compactError'), 'error');
  } finally {
    compactRunning.value = false;
  }
};

onMounted(refresh);
</script>

<template>
  <div class="space-y-6 p-6 fade-in">
    <!-- Header -->
    <div class="flex items-start justify-between gap-4">
      <div class="flex items-center space-x-3">
        <div class="settings-dashboard-icon">
          <Minimize2 :size="20" />
        </div>
        <div>
          <h3 class="settings-dashboard-title">{{ t('compaction.title') }}</h3>
        </div>
      </div>
      <button
        type="button"
        class="ui-button ui-button--primary btn-save-config settings-btn inline-flex min-w-[112px] items-center justify-center gap-2"
        :disabled="isSaving || !isDirty || loading"
        @click="saveConfig"
      >
        <LoaderCircle v-if="isSaving" :size="16" class="animate-spin" />
        <span>{{ isSaving ? t('console.saving') : t('console.saveConfig') }}</span>
      </button>
    </div>

    <p v-if="loadError" class="text-xs" style="color: var(--destructive);">{{ loadError }}</p>

    <!-- Section 1: Budget Status -->
    <div class="bg-card rounded-xl border border-border p-6 shadow-sm space-y-4">
      <div class="flex items-center space-x-2">
        <h4 class="text-sm font-semibold text-foreground">{{ t('compaction.budgetStatus') }}</h4>
      </div>

      <div class="space-y-2">
        <div class="flex justify-between text-xs text-muted-foreground">
          <span>{{ t('compaction.historyTokens') }}</span>
          <span>{{ (budgetStatus?.history_estimated ?? 0).toLocaleString() }} / {{ (budgetStatus?.history_budget ?? 0).toLocaleString() }}</span>
        </div>
        <div class="w-full h-2.5 bg-secondary rounded-full overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-500"
            :class="pressureColor"
            :style="{ width: Math.min(pressurePercent, 100) + '%' }"
          />
        </div>
      </div>

      <div class="flex items-center gap-6 text-sm">
        <div>
          <span class="text-muted-foreground">{{ t('compaction.pressureRatio') }}:</span>
          <span class="ml-1 font-medium text-foreground">{{ pressurePercent }}{{ t('compaction.percent') }}</span>
        </div>
        <div>
          <span class="text-muted-foreground">{{ t('compaction.status') }}:</span>
          <span
            v-if="budgetStatus?.should_compact"
            class="ml-1 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-warning-soft text-warning"
          >
            {{ t('compaction.statusCompact') }}
          </span>
          <span
            v-else
            class="ml-1 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-success-soft text-success"
          >
            {{ t('compaction.statusOk') }}
          </span>
        </div>
      </div>
    </div>

    <!-- Section 2: Configuration -->
    <div v-if="config" class="bg-card rounded-xl border border-border p-6 shadow-sm space-y-5">
      <h4 class="text-sm font-semibold text-foreground">{{ t('compaction.config') }}</h4>

      <label class="settings-label flex items-center space-x-2 cursor-pointer">
        <input v-model="draft.enabled" type="checkbox" class="settings-checkbox" />
        <span>{{ t('compaction.enabled') }}</span>
      </label>

      <!-- max_tokens -->
      <div class="space-y-1">
        <label class="block text-xs font-medium text-muted-foreground uppercase tracking-wider">
          {{ t('compaction.maxTokens') }}
        </label>
        <input
          v-model.number="draft.maxTokens"
          type="number"
          :min="10000"
          :max="500000"
          :step="10000"
          class="ui-input w-full"
        />
        <p class="text-xs text-muted-foreground">{{ t('compaction.maxTokensDesc') }}</p>
      </div>

      <!-- trigger_percent -->
      <div class="space-y-1">
        <label class="block text-xs font-medium text-muted-foreground uppercase tracking-wider">
          {{ t('compaction.thresholdRatio') }}
          <span class="ml-2 text-foreground font-semibold normal-case tracking-normal">{{ draft.triggerPercent }}{{ t('compaction.percent') }}</span>
        </label>
        <input
          v-model.number="draft.triggerPercent"
          type="range"
          :min="10"
          :max="100"
          :step="5"
          class="ui-range w-full"
        />
        <p class="text-xs text-muted-foreground">{{ t('compaction.thresholdRatioDesc') }}</p>
      </div>

      <!-- keep_recent -->
      <div class="space-y-1">
        <label class="block text-xs font-medium text-muted-foreground uppercase tracking-wider">
          {{ t('compaction.keepRecent') }}
        </label>
        <input
          v-model.number="draft.keepRecent"
          type="number"
          :min="1"
          :max="50"
          :step="1"
          class="ui-input w-full"
        />
        <p class="text-xs text-muted-foreground">{{ t('compaction.keepRecentDesc') }}</p>
      </div>
    </div>

    <!-- Section 3: Manual Compaction -->
    <div class="bg-card rounded-xl border border-border p-6 shadow-sm space-y-4">
      <h4 class="text-sm font-semibold text-foreground">{{ t('compaction.manualCompact') }}</h4>
      <div class="flex items-center gap-3">
        <button
          type="button"
          class="ui-button ui-button--ghost inline-flex items-center gap-2 transition-colors"
          :class="budgetStatus && !budgetStatus.should_compact
            ? 'bg-secondary text-muted-foreground cursor-not-allowed'
            : 'bg-info text-foreground hover:bg-info'"
          :disabled="compactRunning || (!!budgetStatus && !budgetStatus.should_compact)"
          :title="budgetStatus && !budgetStatus.should_compact ? t('compaction.noCompactNeeded') : ''"
          @click="runCompact"
        >
          <LoaderCircle v-if="compactRunning" :size="16" class="animate-spin" />
          <Minimize2 v-else :size="16" />
          <span>{{ compactRunning ? t('compaction.compactRunning') : t('compaction.runCompact') }}</span>
        </button>
      </div>
    </div>

    <!-- Reset to Defaults -->
    <div class="flex justify-end">
      <button
        type="button"
        class="ui-button ui-button--ghost inline-flex items-center gap-2  transition-colors"
        @click="resetDefaults"
      >
        <RotateCcw :size="14" />
        <span>{{ t('compaction.resetDefaults') }}</span>
      </button>
    </div>
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
</style>
