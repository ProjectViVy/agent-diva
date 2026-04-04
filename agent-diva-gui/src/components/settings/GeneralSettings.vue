<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue';
import { SlidersHorizontal, MessageSquareText, ShieldCheck, ShieldAlert, FolderTree, DatabaseZap, AlertTriangle } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import { getConfigStatus, startGateway, wipeLocalData, type ConfigStatusReport } from '../../api/desktop';
import { clearAgentDivaLocalStorage, UI_CACHE_KEYS, UI_CACHE_PREFIXES } from '../../utils/localStorageAgentDiva';

const { t } = useI18n();

interface ChatDisplayPrefs {
  autoExpandReasoning: boolean;
  autoExpandToolDetails: boolean;
  showRawMetaByDefault: boolean;
}

const props = defineProps<{
  chatDisplayPrefs: ChatDisplayPrefs;
}>();

const emit = defineEmits<{
  (e: 'save-chat-display-prefs', prefs: ChatDisplayPrefs): void;
}>();

const localPrefs = ref<ChatDisplayPrefs>({ ...props.chatDisplayPrefs });
const statusReport = ref<ConfigStatusReport | null>(null);
const cacheCleared = ref(false);
const preserveLocaleOnWipe = ref(false);
const dangerConfirmInput = ref('');
const wiping = ref(false);
const wipeError = ref<string | null>(null);

watch(
  () => props.chatDisplayPrefs,
  (val) => {
    localPrefs.value = { ...val };
  },
  { deep: true }
);

const emitPrefs = () => {
  emit('save-chat-display-prefs', { ...localPrefs.value });
};

onMounted(async () => {
  try {
    statusReport.value = await getConfigStatus();
  } catch (error) {
    console.error('Failed to load config status:', error);
  }
});

const readyProviders = computed(() =>
  statusReport.value?.providers.filter((item) => item.ready).length ?? 0
);

const readyChannels = computed(() =>
  statusReport.value?.channels.filter((item) => item.enabled && item.ready).length ?? 0
);

const dangerConfirmWord = computed(() => t('general.dangerConfirmWord'));

const dangerConfirmOk = computed(
  () => dangerConfirmInput.value.trim() === dangerConfirmWord.value
);

function clearUiCache() {
  for (const key of UI_CACHE_KEYS) {
    localStorage.removeItem(key);
  }

  const keysToRemove = Object.keys(localStorage).filter((key) =>
    UI_CACHE_PREFIXES.some((prefix) => key.startsWith(prefix))
  );
  for (const key of keysToRemove) {
    localStorage.removeItem(key);
  }

  cacheCleared.value = true;
  window.setTimeout(() => {
    cacheCleared.value = false;
  }, 2500);
}

async function runFullWipe() {
  if (!dangerConfirmOk.value || wiping.value) {
    return;
  }
  wipeError.value = null;
  wiping.value = true;
  try {
    await wipeLocalData();
    clearAgentDivaLocalStorage({ preserveLocale: preserveLocaleOnWipe.value });
    try {
      await startGateway(null);
    } catch (gwErr) {
      console.warn('start_gateway after wipe:', gwErr);
    }
    window.location.reload();
  } catch (e) {
    wipeError.value = e instanceof Error ? e.message : String(e);
  } finally {
    wiping.value = false;
  }
}
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <!-- Header -->
    <div class="flex items-center space-x-3">
      <div class="settings-dashboard-icon">
        <SlidersHorizontal :size="20" />
      </div>
      <div>
        <h3 class="settings-dashboard-title">{{ t('general.title') }}</h3>
        <p class="settings-dashboard-desc">{{ t('general.desc') }}</p>
      </div>
    </div>

    <!-- Chat Settings Card -->
    <div class="settings-section">
      <div class="settings-section-header">
        <MessageSquareText :size="16" />
        <span>{{ t('general.chatSettings') }}</span>
      </div>

      <div class="space-y-3 pl-1">
        <label class="settings-label flex items-center space-x-2 cursor-pointer">
          <input type="checkbox" v-model="localPrefs.autoExpandReasoning" @change="emitPrefs" class="settings-checkbox" />
          <span>{{ t('general.autoExpandReasoning') }}</span>
        </label>
        <label class="settings-label flex items-center space-x-2 cursor-pointer">
          <input type="checkbox" v-model="localPrefs.autoExpandToolDetails" @change="emitPrefs" class="settings-checkbox" />
          <span>{{ t('general.autoExpandToolDetails') }}</span>
        </label>
        <label class="settings-label flex items-center space-x-2 cursor-pointer">
          <input type="checkbox" v-model="localPrefs.showRawMetaByDefault" @change="emitPrefs" class="settings-checkbox" />
          <span>{{ t('general.autoExpandRawMeta') }}</span>
        </label>
      </div>
    </div>

    <!-- Cache Settings Card -->
    <div class="settings-section">
      <div class="settings-section-header">
        <DatabaseZap :size="16" />
        <span>{{ t('general.cacheTitle') }}</span>
      </div>

      <p class="settings-muted mb-3">{{ t('general.cacheDesc') }}</p>

      <div class="flex flex-wrap items-center gap-3">
        <button
          type="button"
          class="settings-btn settings-btn-secondary"
          @click="clearUiCache"
        >
          {{ t('general.clearCache') }}
        </button>
        <span v-if="cacheCleared" class="settings-label text-emerald-600">
          {{ t('general.cacheCleared') }}
        </span>
      </div>
    </div>

    <!-- Runtime Status Card -->
    <div v-if="statusReport" class="settings-section">
      <div class="settings-section-header">
        <FolderTree :size="16" />
        <span>{{ t('general.runtimeStatus') }}</span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-3 mb-4">
        <div class="settings-code-block">
          <div class="flex items-center gap-2 text-xs uppercase tracking-wider settings-muted mb-2">
            <ShieldCheck v-if="statusReport.doctor.ready" :size="14" />
            <ShieldAlert v-else :size="14" />
            <span>{{ t('general.doctorHealth') }}</span>
          </div>
          <div class="text-sm font-semibold" :class="statusReport.doctor.ready ? 'text-emerald-600' : 'text-amber-600'">
            {{ statusReport.doctor.ready ? t('general.healthReady') : t('general.healthAttention') }}
          </div>
        </div>
        <div class="settings-code-block">
          <div class="text-xs uppercase tracking-wider settings-muted mb-2">{{ t('general.providersReady') }}</div>
          <div class="text-sm font-semibold settings-label">{{ readyProviders }} / {{ statusReport.providers.length }}</div>
          <div class="text-xs settings-muted mt-1">{{ statusReport.default_provider || t('providers.unresolved') }}</div>
        </div>
        <div class="settings-code-block">
          <div class="text-xs uppercase tracking-wider settings-muted mb-2">{{ t('general.channelsReady') }}</div>
          <div class="text-sm font-semibold settings-label">{{ readyChannels }} / {{ statusReport.channels.filter((item) => item.enabled).length }}</div>
          <div class="text-xs settings-muted mt-1">{{ t('general.cronJobs', { count: statusReport.cron_jobs }) }}</div>
        </div>
      </div>

      <div class="space-y-2">
        <div class="text-xs uppercase tracking-wider settings-muted">{{ t('general.resolvedPaths') }}</div>
        <div class="settings-code-block space-y-2">
          <div>{{ statusReport.config.config_path }}</div>
          <div>{{ statusReport.config.runtime_dir }}</div>
          <div>{{ statusReport.config.workspace }}</div>
        </div>
      </div>
    </div>

    <!-- Danger Zone -->
    <div class="settings-danger-zone space-y-4">
      <div class="settings-danger-title">
        <AlertTriangle :size="18" class="shrink-0" />
        <span>{{ t('general.dangerZoneTitle') }}</span>
      </div>
      <p class="settings-danger-text leading-relaxed">{{ t('general.dangerZoneDesc') }}</p>
      <p class="text-xs settings-danger-text opacity-80 leading-relaxed">{{ t('general.dangerServiceNote') }}</p>

      <div v-if="statusReport" class="rounded-lg px-3 py-2 space-y-1" style="border: 1px solid var(--danger-bg); background: var(--danger-bg);">
        <div class="text-xs font-medium settings-danger-text">{{ t('general.dangerPathsHint') }}</div>
        <div class="text-xs font-mono settings-label break-all space-y-0.5">
          <div>{{ statusReport.config.config_path }}</div>
          <div>{{ statusReport.config.workspace }}</div>
          <div>{{ statusReport.config.runtime_dir }}</div>
        </div>
      </div>

      <label class="flex items-center gap-2 settings-label cursor-pointer">
        <input v-model="preserveLocaleOnWipe" type="checkbox" class="settings-checkbox" />
        <span>{{ t('general.dangerPreserveLocale') }}</span>
      </label>

      <div class="space-y-1">
        <label class="text-xs font-medium settings-danger-text block">
          {{ t('general.dangerConfirmPrompt', { word: dangerConfirmWord }) }}
        </label>
        <input
          v-model="dangerConfirmInput"
          type="text"
          autocomplete="off"
          class="settings-danger-input"
          :placeholder="dangerConfirmWord"
        />
      </div>

      <div class="flex flex-wrap items-center gap-3">
        <button
          type="button"
          :disabled="!dangerConfirmOk || wiping"
          class="settings-btn settings-btn-danger"
          @click="runFullWipe"
        >
          {{ wiping ? t('general.dangerWiping') : t('general.dangerWipe') }}
        </button>
        <span v-if="wipeError" class="text-sm settings-danger-text">{{ t('general.dangerFailed', { error: wipeError }) }}</span>
      </div>
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
