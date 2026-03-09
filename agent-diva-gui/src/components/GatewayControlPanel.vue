<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  checkHealth,
  getGatewayProcessStatus,
  isTauriRuntime,
  loadRawConfig,
  saveRawConfig,
  startGateway,
  stopGateway,
  tailLogs,
  type GatewayProcessStatus,
} from "../api/desktop";
import { Play, Square, RefreshCcw, FileJson, ScrollText, Activity } from "lucide-vue-next";
import { useI18n } from "vue-i18n";

const { t } = useI18n();

const gatewayStatus = ref<GatewayProcessStatus | null>(null);
const apiHealthy = ref<boolean | null>(null);
const gatewayBusy = ref(false);
const gatewayError = ref("");

const configText = ref("");
const configBusy = ref(false);
const configError = ref("");
const configSavedAt = ref<number | null>(null);

const logLines = ref<string[]>([]);
const logLineCount = ref(200);
const logsBusy = ref(false);
const logsError = ref("");

const lastConfigSavedLabel = computed(() => {
  if (!configSavedAt.value) {
    return "";
  }
  return new Date(configSavedAt.value).toLocaleString();
});

const refreshGatewayStatus = async () => {
  if (!isTauriRuntime()) {
    gatewayStatus.value = {
      running: false,
      details: "browser-preview",
    };
    apiHealthy.value = null;
    return;
  }

  gatewayError.value = "";
  try {
    gatewayStatus.value = await getGatewayProcessStatus();
  } catch (error) {
    gatewayError.value = String(error);
  }

  try {
    apiHealthy.value = await checkHealth();
  } catch {
    apiHealthy.value = false;
  }
};

const runGatewayAction = async (action: "start" | "stop") => {
  gatewayBusy.value = true;
  gatewayError.value = "";
  try {
    if (action === "start") {
      await startGateway();
    } else {
      await stopGateway();
    }
    await refreshGatewayStatus();
  } catch (error) {
    gatewayError.value = String(error);
  } finally {
    gatewayBusy.value = false;
  }
};

const reloadConfig = async () => {
  configBusy.value = true;
  configError.value = "";
  try {
    if (!isTauriRuntime()) {
      configText.value = JSON.stringify({ mock: true }, null, 2);
      return;
    }
    configText.value = await loadRawConfig();
  } catch (error) {
    configError.value = String(error);
  } finally {
    configBusy.value = false;
  }
};

const persistConfig = async () => {
  configBusy.value = true;
  configError.value = "";
  try {
    if (!isTauriRuntime()) {
      configSavedAt.value = Date.now();
      return;
    }
    await saveRawConfig(configText.value);
    configSavedAt.value = Date.now();
    const latest = await loadRawConfig();
    configText.value = latest;
  } catch (error) {
    configError.value = String(error);
  } finally {
    configBusy.value = false;
  }
};

const refreshLogs = async () => {
  logsBusy.value = true;
  logsError.value = "";
  try {
    if (!isTauriRuntime()) {
      logLines.value = ["[mock] gateway logs are only available in Tauri runtime"];
      return;
    }
    logLines.value = await tailLogs(logLineCount.value);
  } catch (error) {
    logsError.value = String(error);
  } finally {
    logsBusy.value = false;
  }
};

onMounted(async () => {
  await Promise.all([refreshGatewayStatus(), reloadConfig(), refreshLogs()]);
});
</script>

<template>
  <div class="space-y-6">
    <section class="bg-white border border-gray-100 rounded-xl p-5 space-y-4 shadow-sm">
      <div class="flex items-center justify-between gap-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-indigo-100 text-indigo-600 flex items-center justify-center">
            <Activity :size="20" />
          </div>
          <div>
            <h3 class="text-lg font-semibold text-gray-800">{{ t('console.gatewayTitle') }}</h3>
            <p class="text-sm text-gray-500">{{ t('console.gatewayDesc') }}</p>
          </div>
        </div>
        <button
          class="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-60"
          :disabled="gatewayBusy"
          @click="refreshGatewayStatus"
        >
          {{ t('console.refreshStatus') }}
        </button>
      </div>

      <div class="grid gap-3 md:grid-cols-2">
        <div class="rounded-lg border border-gray-100 bg-gray-50 px-4 py-3">
          <div class="text-xs text-gray-500">{{ t('console.processState') }}</div>
          <div class="mt-1 text-sm font-medium text-gray-800">
            {{ gatewayStatus?.running ? t('console.gatewayRunning') : t('console.gatewayStopped') }}
          </div>
          <div v-if="gatewayStatus?.pid" class="mt-1 text-xs text-gray-500">
            PID: {{ gatewayStatus.pid }}
          </div>
        </div>

        <div class="rounded-lg border border-gray-100 bg-gray-50 px-4 py-3">
          <div class="text-xs text-gray-500">{{ t('console.managerHealth') }}</div>
          <div class="mt-1 text-sm font-medium text-gray-800">
            {{
              apiHealthy === null
                ? t('console.healthUnknown')
                : apiHealthy
                  ? t('console.healthOnline')
                  : t('console.healthOffline')
            }}
          </div>
          <div v-if="gatewayStatus?.details" class="mt-1 text-xs text-gray-500 break-all">
            {{ gatewayStatus.details }}
          </div>
        </div>
      </div>

      <div v-if="gatewayStatus?.executable_path" class="text-xs text-gray-500 break-all">
        {{ gatewayStatus.executable_path }}
      </div>

      <div class="flex flex-wrap gap-2">
        <button
          class="inline-flex items-center gap-2 px-3 py-2 text-sm rounded-lg bg-indigo-600 text-white hover:bg-indigo-700 disabled:opacity-60"
          :disabled="gatewayBusy || gatewayStatus?.running"
          @click="runGatewayAction('start')"
        >
          <Play :size="16" />
          {{ t('console.startGateway') }}
        </button>
        <button
          class="inline-flex items-center gap-2 px-3 py-2 text-sm rounded-lg border border-gray-200 text-gray-700 hover:bg-gray-50 disabled:opacity-60"
          :disabled="gatewayBusy || !gatewayStatus?.running"
          @click="runGatewayAction('stop')"
        >
          <Square :size="16" />
          {{ t('console.stopGateway') }}
        </button>
      </div>

      <p v-if="gatewayError" class="text-xs text-red-600 break-words">
        {{ gatewayError }}
      </p>
    </section>

    <section class="bg-white border border-gray-100 rounded-xl p-5 space-y-4 shadow-sm">
      <div class="flex items-center justify-between gap-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-emerald-100 text-emerald-600 flex items-center justify-center">
            <FileJson :size="20" />
          </div>
          <div>
            <h3 class="text-lg font-semibold text-gray-800">{{ t('console.configTitle') }}</h3>
            <p class="text-sm text-gray-500">{{ t('console.configDesc') }}</p>
          </div>
        </div>
        <div class="flex gap-2">
          <button
            class="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-60"
            :disabled="configBusy"
            @click="reloadConfig"
          >
            {{ t('console.reloadConfig') }}
          </button>
          <button
            class="px-3 py-2 text-xs rounded-lg bg-emerald-600 text-white hover:bg-emerald-700 disabled:opacity-60"
            :disabled="configBusy"
            @click="persistConfig"
          >
            {{ t('console.saveConfig') }}
          </button>
        </div>
      </div>

      <textarea
        v-model="configText"
        class="w-full min-h-[260px] rounded-xl border border-gray-200 px-4 py-3 font-mono text-xs text-gray-800 bg-gray-50 focus:outline-none focus:ring-2 focus:ring-emerald-200"
        spellcheck="false"
      />

      <div class="flex items-center justify-between gap-4 text-xs">
        <span class="text-gray-500">
          {{ lastConfigSavedLabel ? `${t('console.savedAt')}: ${lastConfigSavedLabel}` : t('console.jsonEditorHint') }}
        </span>
        <span v-if="configBusy" class="text-gray-500">{{ t('console.saving') }}</span>
      </div>

      <p v-if="configError" class="text-xs text-red-600 break-words">
        {{ configError }}
      </p>
    </section>

    <section class="bg-white border border-gray-100 rounded-xl p-5 space-y-4 shadow-sm">
      <div class="flex items-center justify-between gap-4">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-amber-100 text-amber-600 flex items-center justify-center">
            <ScrollText :size="20" />
          </div>
          <div>
            <h3 class="text-lg font-semibold text-gray-800">{{ t('console.logsTitle') }}</h3>
            <p class="text-sm text-gray-500">{{ t('console.logsDesc') }}</p>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <select
            v-model="logLineCount"
            class="px-2 py-2 text-xs rounded-lg border border-gray-200 bg-white text-gray-700"
          >
            <option :value="100">100</option>
            <option :value="200">200</option>
            <option :value="500">500</option>
          </select>
          <button
            class="inline-flex items-center gap-2 px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-60"
            :disabled="logsBusy"
            @click="refreshLogs"
          >
            <RefreshCcw :size="14" />
            {{ t('console.refreshLogs') }}
          </button>
        </div>
      </div>

      <div class="rounded-xl border border-gray-200 bg-slate-950 text-slate-100 min-h-[220px] max-h-[420px] overflow-auto">
        <div v-if="logLines.length === 0" class="px-4 py-6 text-sm text-slate-400">
          {{ t('console.noLogs') }}
        </div>
        <pre v-else class="px-4 py-4 text-xs leading-5 whitespace-pre-wrap break-words"><template v-for="(line, index) in logLines" :key="`${index}-${line}`"><span :class="{
            'text-red-300': /error/i.test(line),
            'text-amber-300': /warn/i.test(line),
            'text-emerald-300': /info/i.test(line),
            'text-slate-100': !/error|warn|info/i.test(line)
          }">{{ line }}</span>
{{ index < logLines.length - 1 ? '\n' : '' }}</template></pre>
      </div>

      <p v-if="logsError" class="text-xs text-red-600 break-words">
        {{ logsError }}
      </p>
    </section>
  </div>
</template>
