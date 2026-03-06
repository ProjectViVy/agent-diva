<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { SlidersHorizontal, MessageSquareText, ServerCog } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';
import {
  getRuntimeInfo,
  getServiceStatus,
  installService,
  isTauriRuntime,
  startService,
  stopService,
  uninstallService,
  type RuntimeInfo,
  type ServiceStatusPayload,
} from '../../api/desktop';

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
const runtimeInfo = ref<RuntimeInfo | null>(null);
const serviceStatus = ref<ServiceStatusPayload | null>(null);
const serviceBusy = ref(false);
const serviceError = ref('');
const hasTauriRuntime = isTauriRuntime();

const isBundledApp = computed(() => runtimeInfo.value?.is_bundled === true);
const servicePanelEnabled = computed(
  () => isBundledApp.value && ['windows', 'linux', 'macos'].includes(runtimeInfo.value?.platform || '')
);
const serviceActionsEnabled = computed(() =>
  runtimeInfo.value?.platform === 'windows' || runtimeInfo.value?.platform === 'linux'
);
const runtimeModeLabel = computed(() =>
  runtimeInfo.value?.is_bundled ? t('general.runtimeBundled') : t('general.runtimeDev')
);
const platformLabel = computed(() => {
  const platform = runtimeInfo.value?.platform || 'unknown';
  if (platform === 'windows') return 'Windows';
  if (platform === 'linux') return 'Linux';
  if (platform === 'macos') return 'macOS';
  if (platform === 'browser') return t('general.browserPreview');
  return platform;
});
const serviceStateLabel = computed(() => {
  if (!serviceStatus.value) {
    return t('general.serviceStateUnknown');
  }
  if (!serviceStatus.value.installed) {
    return t('general.serviceStateNotInstalled');
  }
  if (serviceStatus.value.running) {
    return t('general.serviceStateRunning');
  }
  return t('general.serviceStateInstalled');
});
const installLabel = computed(() =>
  runtimeInfo.value?.platform === 'linux'
    ? t('general.installSystemd')
    : runtimeInfo.value?.platform === 'macos'
      ? t('general.installLaunchd')
      : t('general.installService')
);
const uninstallLabel = computed(() =>
  runtimeInfo.value?.platform === 'linux'
    ? t('general.uninstallSystemd')
    : runtimeInfo.value?.platform === 'macos'
      ? t('general.uninstallLaunchd')
      : t('general.uninstallService')
);

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

const loadRuntimeInfo = async () => {
  try {
    runtimeInfo.value = await getRuntimeInfo();
  } catch (error) {
    serviceError.value = String(error);
  }
};

const refreshServiceStatus = async () => {
  if (!hasTauriRuntime || !servicePanelEnabled.value) {
    return;
  }

  serviceError.value = '';
  try {
    serviceStatus.value = await getServiceStatus();
  } catch (error) {
    serviceStatus.value = null;
    serviceError.value = String(error);
  }
};

const runServiceAction = async (
  command: 'install_service' | 'uninstall_service' | 'start_service' | 'stop_service'
) => {
  if (!serviceActionsEnabled.value) {
    serviceError.value = t('general.servicePlatformPending');
    return;
  }
  serviceBusy.value = true;
  serviceError.value = '';
  try {
    if (command === 'install_service') {
      await installService();
    } else if (command === 'uninstall_service') {
      await uninstallService();
    } else if (command === 'start_service') {
      await startService();
    } else {
      await stopService();
    }
    await refreshServiceStatus();
  } catch (error) {
    serviceError.value = String(error);
  } finally {
    serviceBusy.value = false;
  }
};

onMounted(async () => {
  if (!hasTauriRuntime) {
    runtimeInfo.value = {
      platform: 'browser',
      is_bundled: false,
      resource_dir: null
    };
    return;
  }
  await loadRuntimeInfo();
  if (servicePanelEnabled.value) {
    await refreshServiceStatus();
  }
});
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
      <div class="flex items-center justify-between gap-4">
        <div class="flex items-center gap-2">
          <ServerCog :size="16" class="text-violet-500" />
          <div>
            <h4 class="text-sm font-semibold text-gray-800">{{ t('general.serviceTitle') }}</h4>
            <p class="text-xs text-gray-500">
              {{ t('general.runtimeLabel') }}: {{ runtimeModeLabel }} · {{ t('general.platformLabel') }}: {{ platformLabel }}
            </p>
          </div>
        </div>
        <button
          class="px-3 py-1.5 text-xs rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-60"
          :disabled="serviceBusy || !servicePanelEnabled"
          @click="refreshServiceStatus"
        >
          {{ t('general.refreshService') }}
        </button>
      </div>

      <div v-if="servicePanelEnabled" class="space-y-3">
        <div class="rounded-lg border border-gray-100 bg-gray-50 px-4 py-3 text-sm text-gray-700 space-y-1">
          <p>
            {{ t('general.serviceState') }}:
            <span class="font-medium">{{ serviceStateLabel }}</span>
          </p>
          <p>
            {{ t('general.serviceInstalled') }}:
            <span class="font-medium">{{ serviceStatus?.installed ? t('general.yes') : t('general.no') }}</span>
          </p>
          <p>
            {{ t('general.serviceRunning') }}:
            <span class="font-medium">{{ serviceStatus?.running ? t('general.yes') : t('general.no') }}</span>
          </p>
          <p v-if="serviceStatus?.details" class="text-xs text-gray-500 break-words">
            {{ serviceStatus.details }}
          </p>
          <p v-if="serviceStatus?.executable_path" class="text-xs text-gray-500 break-all">
            {{ serviceStatus.executable_path }}
          </p>
        </div>

        <div class="flex flex-wrap gap-2">
          <button
            class="px-3 py-2 text-sm rounded-lg bg-violet-600 text-white hover:bg-violet-700 disabled:opacity-60"
            :disabled="serviceBusy || !serviceActionsEnabled || !!serviceStatus?.installed"
            @click="runServiceAction('install_service')"
          >
            {{ installLabel }}
          </button>
          <button
            class="px-3 py-2 text-sm rounded-lg border border-gray-200 text-gray-700 hover:bg-gray-50 disabled:opacity-60"
            :disabled="serviceBusy || !serviceActionsEnabled || !serviceStatus?.installed"
            @click="runServiceAction('start_service')"
          >
            {{ t('general.startService') }}
          </button>
          <button
            class="px-3 py-2 text-sm rounded-lg border border-gray-200 text-gray-700 hover:bg-gray-50 disabled:opacity-60"
            :disabled="serviceBusy || !serviceActionsEnabled || !serviceStatus?.installed"
            @click="runServiceAction('stop_service')"
          >
            {{ t('general.stopService') }}
          </button>
          <button
            class="px-3 py-2 text-sm rounded-lg border border-red-200 text-red-600 hover:bg-red-50 disabled:opacity-60"
            :disabled="serviceBusy || !serviceActionsEnabled || !serviceStatus?.installed"
            @click="runServiceAction('uninstall_service')"
          >
            {{ uninstallLabel }}
          </button>
        </div>

        <p
          v-if="runtimeInfo?.platform === 'macos'"
          class="text-xs text-amber-600"
        >
          {{ t('general.servicePlatformPending') }}
        </p>

        <p v-if="serviceError" class="text-xs text-red-600 break-words">
          {{ serviceError }}
        </p>
      </div>

      <div v-else class="space-y-2">
        <h4 class="text-sm font-semibold text-gray-700">
          {{ t('general.serviceOnlyBundled') }}
        </h4>
        <p class="text-xs text-gray-500">
          {{ t('general.serviceOnlyBundledDesc') }}
        </p>
        <p v-if="serviceError" class="text-xs text-red-600 break-words">
          {{ serviceError }}
        </p>
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
