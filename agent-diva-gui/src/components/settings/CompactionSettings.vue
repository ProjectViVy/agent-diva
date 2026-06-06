<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { Minimize2, LoaderCircle, RefreshCw, RotateCcw } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { invoke } from '@tauri-apps/api/core';
import { showAppToast } from '../../utils/appToast';

const { t } = useI18n();

interface CompactionConfig {
  max_tokens: number;
  compact_threshold_ratio: number;
  keep_recent_count: number;
}

interface BudgetStatus {
  history_estimated: number;
  history_budget: number;
  pressure_ratio: number;
  should_compact: boolean;
}

const STORAGE_KEY = 'agent-diva-compaction-config';

const DEFAULT_CONFIG: CompactionConfig = {
  max_tokens: 180000,
  compact_threshold_ratio: 0.80,
  keep_recent_count: 10,
};

const config = ref<CompactionConfig>({ ...DEFAULT_CONFIG });
const budgetStatus = ref<BudgetStatus | null>(null);
const budgetUnavailable = ref(false);
const budgetLoading = ref(false);
const compactRunning = ref(false);

let saveTimer: ReturnType<typeof setTimeout> | null = null;

const pressurePercent = computed(() => {
  if (!budgetStatus.value) return 0;
  return Math.round(budgetStatus.value.pressure_ratio * 100);
});

const pressureColor = computed(() => {
  const p = pressurePercent.value;
  if (p < 60) return 'bg-green-500';
  if (p < 80) return 'bg-yellow-500';
  return 'bg-red-500';
});

const thresholdPercent = computed({
  get: () => Math.round(config.value.compact_threshold_ratio * 100),
  set: (val: number) => {
    config.value.compact_threshold_ratio = val / 100;
  },
});

const loadBudgetStatus = async () => {
  budgetLoading.value = true;
  budgetUnavailable.value = false;
  try {
    const data = await invoke<BudgetStatus>('get_budget_status');
    budgetStatus.value = data;
  } catch {
    budgetStatus.value = null;
    budgetUnavailable.value = true;
  } finally {
    budgetLoading.value = false;
  }
};

const loadConfig = () => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const saved = JSON.parse(raw) as Partial<CompactionConfig>;
      config.value = {
        max_tokens: saved.max_tokens ?? DEFAULT_CONFIG.max_tokens,
        compact_threshold_ratio: saved.compact_threshold_ratio ?? DEFAULT_CONFIG.compact_threshold_ratio,
        keep_recent_count: saved.keep_recent_count ?? DEFAULT_CONFIG.keep_recent_count,
      };
    }
  } catch {
    config.value = { ...DEFAULT_CONFIG };
  }
};

const saveConfig = () => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(config.value));
  } catch {
    // ignore storage errors
  }
};

const resetDefaults = () => {
  config.value = { ...DEFAULT_CONFIG };
  saveConfig();
  showAppToast(t('compaction.resetDefaults'), 'success');
};

const runCompact = async () => {
  compactRunning.value = true;
  try {
    await invoke('send_message', {
      message: '/compact',
      channel: null,
      chatId: null,
      attachments: null,
      streamRequestId: crypto.randomUUID(),
    });
    showAppToast(t('compaction.compactSuccess'), 'success');
    await loadBudgetStatus();
  } catch {
    showAppToast(t('compaction.compactError'), 'error');
  } finally {
    compactRunning.value = false;
  }
};

watch(
  config,
  () => {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(saveConfig, 500);
  },
  { deep: true }
);

onMounted(() => {
  loadConfig();
  loadBudgetStatus();
});
</script>

<template>
  <div class="space-y-6 p-6 fade-in">
    <!-- Header -->
    <div class="flex items-center space-x-3">
      <div class="settings-dashboard-icon">
        <Minimize2 :size="20" />
      </div>
      <div>
        <h3 class="settings-dashboard-title">{{ t('compaction.title') }}</h3>
      </div>
    </div>

    <!-- Section 1: Budget Status -->
    <div class="bg-white rounded-xl border border-gray-200 p-6 shadow-sm space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center space-x-2">
          <h4 class="text-sm font-semibold text-gray-700">{{ t('compaction.budgetStatus') }}</h4>
        </div>
        <button
          type="button"
          class="p-1.5 rounded-lg hover:bg-gray-100 text-gray-500 transition-colors"
          :disabled="budgetLoading"
          @click="loadBudgetStatus"
        >
          <RefreshCw :size="16" :class="{ 'animate-spin': budgetLoading }" />
        </button>
      </div>

      <!-- Unavailable -->
      <div v-if="budgetUnavailable" class="text-sm text-gray-500 py-4 text-center">
        {{ t('compaction.unavailable') }}
      </div>

      <!-- Budget Details -->
      <template v-else-if="budgetStatus">
        <!-- Progress Bar -->
        <div class="space-y-2">
          <div class="flex justify-between text-xs text-gray-500">
            <span>{{ t('compaction.historyTokens') }}</span>
            <span>{{ budgetStatus.history_estimated.toLocaleString() }} / {{ budgetStatus.history_budget.toLocaleString() }}</span>
          </div>
          <div class="w-full h-2.5 bg-gray-200 rounded-full overflow-hidden">
            <div
              class="h-full rounded-full transition-all duration-500"
              :class="pressureColor"
              :style="{ width: Math.min(pressurePercent, 100) + '%' }"
            />
          </div>
        </div>

        <!-- Stats Row -->
        <div class="flex items-center gap-6 text-sm">
          <div>
            <span class="text-gray-500">{{ t('compaction.pressureRatio') }}:</span>
            <span class="ml-1 font-medium text-gray-700">{{ pressurePercent }}{{ t('compaction.percent') }}</span>
          </div>
          <div>
            <span class="text-gray-500">{{ t('compaction.status') }}:</span>
            <span
              v-if="budgetStatus.should_compact"
              class="ml-1 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-orange-100 text-orange-700"
            >
              {{ t('compaction.statusCompact') }}
            </span>
            <span
              v-else
              class="ml-1 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700"
            >
              {{ t('compaction.statusOk') }}
            </span>
          </div>
        </div>
      </template>

      <!-- Loading state -->
      <div v-else-if="budgetLoading" class="flex items-center justify-center py-6">
        <LoaderCircle :size="20" class="animate-spin text-gray-400" />
      </div>
    </div>

    <!-- Section 2: Configuration -->
    <div class="bg-white rounded-xl border border-gray-200 p-6 shadow-sm space-y-5">
      <h4 class="text-sm font-semibold text-gray-700">{{ t('compaction.config') }}</h4>

      <!-- max_tokens -->
      <div class="space-y-1">
        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">
          {{ t('compaction.maxTokens') }}
        </label>
        <input
          v-model.number="config.max_tokens"
          type="number"
          :min="10000"
          :max="500000"
          :step="10000"
          class="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
        <p class="text-xs text-gray-400">{{ t('compaction.maxTokensDesc') }}</p>
      </div>

      <!-- compact_threshold_ratio -->
      <div class="space-y-1">
        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">
          {{ t('compaction.thresholdRatio') }}
          <span class="ml-2 text-gray-700 font-semibold normal-case tracking-normal">{{ thresholdPercent }}{{ t('compaction.percent') }}</span>
        </label>
        <input
          v-model.number="thresholdPercent"
          type="range"
          :min="10"
          :max="100"
          :step="5"
          class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-500"
        />
        <p class="text-xs text-gray-400">{{ t('compaction.thresholdRatioDesc') }}</p>
      </div>

      <!-- keep_recent_count -->
      <div class="space-y-1">
        <label class="block text-xs font-medium text-gray-500 uppercase tracking-wider">
          {{ t('compaction.keepRecent') }}
        </label>
        <input
          v-model.number="config.keep_recent_count"
          type="number"
          :min="1"
          :max="50"
          :step="1"
          class="w-full rounded-lg border border-gray-300 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
        <p class="text-xs text-gray-400">{{ t('compaction.keepRecentDesc') }}</p>
      </div>
    </div>

    <!-- Section 3: Manual Compaction -->
    <div class="bg-white rounded-xl border border-gray-200 p-6 shadow-sm space-y-4">
      <h4 class="text-sm font-semibold text-gray-700">{{ t('compaction.manualCompact') }}</h4>
      <div class="flex items-center gap-3">
        <button
          type="button"
          class="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-colors"
          :class="budgetStatus && !budgetStatus.should_compact
            ? 'bg-gray-100 text-gray-400 cursor-not-allowed'
            : 'bg-blue-600 text-white hover:bg-blue-700'"
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
        class="inline-flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium text-gray-600 bg-gray-100 hover:bg-gray-200 transition-colors"
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