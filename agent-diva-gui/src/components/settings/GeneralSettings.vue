<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue';
import { SlidersHorizontal, MessageSquareText, ServerCog, ShieldCheck, ShieldAlert, FolderTree, DatabaseZap } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import GatewayControlPanel from '../GatewayControlPanel.vue';
import { getConfigStatus, type ConfigStatusReport } from '../../api/desktop';

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

const CACHE_KEYS = ['agent-diva-saved-models', 'agent-diva-history-prefs'];
const CACHE_PREFIXES = ['agent-diva-session-cache:'];

function clearUiCache() {
  for (const key of CACHE_KEYS) {
    localStorage.removeItem(key);
  }

  const keysToRemove = Object.keys(localStorage).filter((key) =>
    CACHE_PREFIXES.some((prefix) => key.startsWith(prefix))
  );
  for (const key of keysToRemove) {
    localStorage.removeItem(key);
  }

  cacheCleared.value = true;
  window.setTimeout(() => {
    cacheCleared.value = false;
  }, 2500);
}
</script>

<template>
  <div class="p-6 space-y-6 fade-in">
    <div class="flex items-center space-x-3">
      <div class="w-10 h-10 rounded-lg bg-violet-100 text-violet-600 flex items-center justify-center">
        <SlidersHorizontal :size="20" />
      </div>
      <div>
        <h3 class="text-lg font-bold text-gray-800">{{ t('general.title') }}</h3>
        <p class="text-sm text-gray-500">{{ t('general.desc') }}</p>
      </div>
    </div>

    <div class="bg-white border border-gray-100 rounded-xl p-4 space-y-4">
      <div class="flex items-center space-x-2 text-gray-700">
        <MessageSquareText :size="16" class="text-violet-500" />
        <span class="text-sm font-semibold">{{ t('general.chatSettings') }}</span>
      </div>

      <div class="space-y-3 pl-1">
        <label class="text-sm text-gray-700 flex items-center space-x-2">
          <input type="checkbox" v-model="localPrefs.autoExpandReasoning" @change="emitPrefs" />
          <span>{{ t('general.autoExpandReasoning') }}</span>
        </label>
        <label class="text-sm text-gray-700 flex items-center space-x-2">
          <input type="checkbox" v-model="localPrefs.autoExpandToolDetails" @change="emitPrefs" />
          <span>{{ t('general.autoExpandToolDetails') }}</span>
        </label>
        <label class="text-sm text-gray-700 flex items-center space-x-2">
          <input type="checkbox" v-model="localPrefs.showRawMetaByDefault" @change="emitPrefs" />
          <span>{{ t('general.autoExpandRawMeta') }}</span>
        </label>
      </div>
    </div>

    <div class="bg-white border border-gray-100 rounded-xl p-4 space-y-4">
      <div class="flex items-center space-x-2 text-gray-700">
        <DatabaseZap :size="16" class="text-violet-500" />
        <span class="text-sm font-semibold">{{ t('general.cacheTitle') }}</span>
      </div>

      <p class="text-sm text-gray-500">{{ t('general.cacheDesc') }}</p>

      <div class="flex flex-wrap items-center gap-3">
        <button
          type="button"
          class="inline-flex items-center rounded-lg bg-gray-900 px-4 py-2 text-sm font-medium text-white transition hover:bg-gray-800"
          @click="clearUiCache"
        >
          {{ t('general.clearCache') }}
        </button>
        <span v-if="cacheCleared" class="text-sm text-emerald-600">
          {{ t('general.cacheCleared') }}
        </span>
      </div>
    </div>

    <div v-if="statusReport" class="bg-white border border-gray-100 rounded-xl p-4 space-y-4">
      <div class="flex items-center space-x-2 text-gray-700">
        <FolderTree :size="16" class="text-violet-500" />
        <span class="text-sm font-semibold">{{ t('general.runtimeStatus') }}</span>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
        <div class="rounded-xl border border-gray-200 bg-gray-50 px-4 py-3">
          <div class="flex items-center gap-2 text-xs uppercase tracking-wider text-gray-400">
            <ShieldCheck v-if="statusReport.doctor.ready" :size="14" />
            <ShieldAlert v-else :size="14" />
            <span>{{ t('general.doctorHealth') }}</span>
          </div>
          <div class="mt-2 text-sm font-semibold" :class="statusReport.doctor.ready ? 'text-emerald-700' : 'text-amber-700'">
            {{ statusReport.doctor.ready ? t('general.healthReady') : t('general.healthAttention') }}
          </div>
        </div>
        <div class="rounded-xl border border-gray-200 bg-gray-50 px-4 py-3">
          <div class="text-xs uppercase tracking-wider text-gray-400">{{ t('general.providersReady') }}</div>
          <div class="mt-2 text-sm font-semibold text-gray-800">{{ readyProviders }} / {{ statusReport.providers.length }}</div>
          <div class="mt-1 text-xs text-gray-500">{{ statusReport.default_provider || t('providers.unresolved') }}</div>
        </div>
        <div class="rounded-xl border border-gray-200 bg-gray-50 px-4 py-3">
          <div class="text-xs uppercase tracking-wider text-gray-400">{{ t('general.channelsReady') }}</div>
          <div class="mt-2 text-sm font-semibold text-gray-800">{{ readyChannels }} / {{ statusReport.channels.filter((item) => item.enabled).length }}</div>
          <div class="mt-1 text-xs text-gray-500">{{ t('general.cronJobs', { count: statusReport.cron_jobs }) }}</div>
        </div>
      </div>

      <div class="space-y-2">
        <div class="text-xs uppercase tracking-wider text-gray-400">{{ t('general.resolvedPaths') }}</div>
        <div class="rounded-xl border border-gray-200 bg-gray-50 px-4 py-3 space-y-2 text-xs font-mono text-gray-700">
          <div>{{ statusReport.config.config_path }}</div>
          <div>{{ statusReport.config.runtime_dir }}</div>
          <div>{{ statusReport.config.workspace }}</div>
        </div>
      </div>
    </div>

    <div class="space-y-4">
      <div class="flex items-center gap-2 px-1">
        <ServerCog :size="16" class="text-violet-500" />
        <div>
          <h4 class="text-sm font-semibold text-gray-800">{{ t('general.serviceTitle') }}</h4>
          <p class="text-xs text-gray-500">{{ t('console.gatewayDesc') }}</p>
        </div>
      </div>

      <GatewayControlPanel />
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
