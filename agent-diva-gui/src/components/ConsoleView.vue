<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Activity, Server, Zap } from 'lucide-vue-next';

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
} from '../api/desktop';
import StatusPanel from './console/StatusPanel.vue';
import ConfigEditor from './console/ConfigEditor.vue';
import LogPanel from './console/LogPanel.vue';
import TokenStatsPanel from './console/TokenStatsPanel.vue';

const { t } = useI18n();

// Gateway state
const gatewayStatus = ref<GatewayProcessStatus | null>(null);
const apiHealthy = ref<boolean | null>(null);
const gatewayBusy = ref(false);
const gatewayError = ref('');

// Config state
const configText = ref('');
const configBusy = ref(false);
const configError = ref('');
const configSavedAt = ref<number | null>(null);

// Logs state
const logLines = ref<string[]>([]);
const logLineCount = ref(200);
const logsBusy = ref(false);
const logsError = ref('');

// Gateway actions
const refreshGatewayStatus = async () => {
  if (!isTauriRuntime()) {
    gatewayStatus.value = {
      running: false,
      details: 'browser-preview',
    };
    apiHealthy.value = null;
    return;
  }

  gatewayError.value = '';
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

const runGatewayAction = async (action: 'start' | 'stop') => {
  gatewayBusy.value = true;
  gatewayError.value = '';
  try {
    if (action === 'start') {
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

// Config actions
const reloadConfig = async () => {
  configBusy.value = true;
  configError.value = '';
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
  configError.value = '';
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

// Logs actions
const fetchLogLines = async (opts?: { withSpinner?: boolean }) => {
  const withSpinner = opts?.withSpinner ?? false;
  if (withSpinner) {
    logsBusy.value = true;
  }
  logsError.value = '';
  try {
    if (!isTauriRuntime()) {
      logLines.value = ['[mock] gateway logs are only available in Tauri runtime'];
      return;
    }
    logLines.value = await tailLogs(logLineCount.value);
  } catch (error) {
    logsError.value = String(error);
  } finally {
    if (withSpinner) {
      logsBusy.value = false;
    }
  }
};

const refreshLogs = () => fetchLogLines({ withSpinner: true });

const updateLogLineCount = (value: number) => {
  logLineCount.value = value;
  fetchLogLines({ withSpinner: true });
};

// Polling
let logPollId: number | undefined;
let healthPollId: number | undefined;

onMounted(async () => {
  await Promise.all([
    refreshGatewayStatus(),
    reloadConfig(),
    fetchLogLines({ withSpinner: true }),
  ]);

  if (isTauriRuntime()) {
    logPollId = window.setInterval(() => {
      void fetchLogLines();
    }, 2000) as number;

    healthPollId = window.setInterval(() => {
      void refreshGatewayStatus();
    }, 5000) as number;
  }
});

onUnmounted(() => {
  if (logPollId !== undefined) {
    window.clearInterval(logPollId);
  }
  if (healthPollId !== undefined) {
    window.clearInterval(healthPollId);
  }
});
</script>

<template>
  <div class="console-view h-full min-h-0 overflow-y-auto">
    <div class="mx-auto max-w-5xl space-y-6 p-6">
      <!-- Header with gradient -->
      <section class="console-header">
        <div class="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
          <div class="min-w-0 flex-1 space-y-1.5">
            <h2 class="text-[30px] font-bold leading-none tracking-tight text-white">
              {{ t('console.gatewayTitle') }}
            </h2>
          </div>
        </div>
      </section>

      <!-- Status Panel Section -->
      <section class="console-section">
        <div class="flex items-center gap-3">
          <div class="console-section-icon console-section-icon--blue">
            <Activity :size="20" />
          </div>
          <div>
            <h3 class="console-section-title">{{ t('console.gatewayTitle') }}</h3>
            <p class="console-section-desc">{{ t('console.gatewayDesc') }}</p>
          </div>
        </div>

        <StatusPanel
          :gateway-status="gatewayStatus"
          :api-healthy="apiHealthy"
          :loading="false"
          :busy="gatewayBusy"
          @start="runGatewayAction('start')"
          @stop="runGatewayAction('stop')"
          @refresh="refreshGatewayStatus"
        />

        <p v-if="gatewayError" class="console-error">
          {{ gatewayError }}
        </p>
      </section>

      <!-- Config Editor Section -->
      <section class="console-section">
        <ConfigEditor
          :config-text="configText"
          :loading="configBusy"
          :saving="configBusy"
          :error="configError"
          :saved-at="configSavedAt"
          @update:config-text="configText = $event"
          @reload="reloadConfig"
          @save="persistConfig"
        />
      </section>

      <!-- Token Stats Section -->
      <section class="console-section">
        <div class="flex items-center gap-3 mb-4">
          <div class="console-section-icon console-section-icon--purple">
            <Zap :size="20" />
          </div>
          <div>
            <h3 class="console-section-title">{{ t('tokenStats.title', 'Token Statistics') }}</h3>
            <p class="console-section-desc">{{ t('tokenStats.desc', 'Monitor AI model usage and costs') }}</p>
          </div>
        </div>

        <TokenStatsPanel />
      </section>

      <!-- Log Panel Section -->
      <section class="console-section">
        <div class="flex items-center gap-3 mb-4">
          <div class="console-section-icon console-section-icon--amber">
            <Server :size="20" />
          </div>
          <div>
            <h3 class="console-section-title">{{ t('console.logsTitle') }}</h3>
            <p class="console-section-desc">{{ t('console.logsDesc') }}</p>
          </div>
        </div>

        <LogPanel
          :log-lines="logLines"
          :loading="logsBusy"
          :error="logsError"
          :max-lines="logLineCount"
          @refresh="refreshLogs"
          @update:max-lines="updateLogLineCount"
        />
      </section>
    </div>
  </div>
</template>

<style scoped>
.console-header {
  @apply rounded-[28px] bg-gradient-to-r from-indigo-500 via-purple-500 to-pink-500 px-6 py-5 text-white shadow-lg;
}

/* Console section card */
.console-section {
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 1.5rem;
  margin-bottom: 1.5rem;
  box-shadow: var(--shadow);
}

.console-section-icon {
  width: 2.5rem;
  height: 2.5rem;
  border-radius: var(--radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
}

.console-section-icon--blue {
  background: var(--accent-bg-light);
  color: var(--accent);
}

.console-section-icon--amber {
  background: var(--warning-bg, rgba(245, 158, 11, 0.15));
  color: var(--warning);
}

.console-section-icon--purple {
  background: rgba(139, 92, 246, 0.15);
  color: #8b5cf6;
}

.console-section-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.25rem;
}

.console-section-desc {
  font-size: 0.875rem;
  color: var(--text-muted);
}

.console-section-header {
  margin-bottom: 1.25rem;
}

.console-error {
  font-size: 0.75rem;
  color: var(--danger);
  word-break: break-all;
  margin-top: 1rem;
}
</style>