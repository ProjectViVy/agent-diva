<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { Activity, Play, Square, ShieldCheck, ShieldAlert, Server, Cpu } from 'lucide-vue-next';

const { t } = useI18n();

export interface GatewayProcessStatus {
  running: boolean;
  pid?: number | null;
  details?: string | null;
  executable_path?: string | null;
}

const props = defineProps<{
  gatewayStatus: GatewayProcessStatus | null;
  apiHealthy: boolean | null;
  loading?: boolean;
  busy?: boolean;
}>();

const emit = defineEmits<{
  (e: 'start'): void;
  (e: 'stop'): void;
  (e: 'refresh'): void;
}>();

const isRunning = computed(() => props.gatewayStatus?.running ?? false);
const isOnline = computed(() => props.apiHealthy === true);
const isOffline = computed(() => props.apiHealthy === false);
const isUnknown = computed(() => props.apiHealthy === null);

function startGateway() {
  emit('start');
}

function stopGateway() {
  emit('stop');
}

function refresh() {
  emit('refresh');
}
</script>

<template>
  <div class="status-panel space-y-4">
    <!-- Status grid -->
    <div class="grid gap-3 md:grid-cols-2">
      <!-- Gateway process status -->
      <div class="status-card">
        <div class="status-card-header">
          <div class="status-card-icon bg-indigo-500/10 text-indigo-500">
            <Server :size="16" />
          </div>
          <span class="status-card-title">{{ t('console.processState') }}</span>
        </div>
        <div class="status-card-body">
          <div class="flex items-center gap-2">
            <span
              class="status-dot"
              :class="isRunning ? 'status-dot--running' : 'status-dot--stopped'"
            />
            <span class="status-value">
              {{ isRunning ? t('console.gatewayRunning') : t('console.gatewayStopped') }}
            </span>
          </div>
          <p v-if="gatewayStatus?.pid" class="status-detail mt-1">
            PID: {{ gatewayStatus.pid }}
          </p>
          <p v-if="gatewayStatus?.details" class="status-detail">
            {{ gatewayStatus.details }}
          </p>
        </div>
      </div>

      <!-- Manager health status -->
      <div class="status-card">
        <div class="status-card-header">
          <div
            class="status-card-icon"
            :class="isOnline ? 'bg-emerald-500/10 text-emerald-500' : 'bg-amber-500/10 text-amber-500'"
          >
            <ShieldCheck v-if="isOnline" :size="16" />
            <ShieldAlert v-else :size="16" />
          </div>
          <span class="status-card-title">{{ t('console.managerHealth') }}</span>
        </div>
        <div class="status-card-body">
          <div class="flex items-center gap-2">
            <span
              class="status-dot"
              :class="{
                'status-dot--online': isOnline,
                'status-dot--offline': isOffline,
                'status-dot--unknown': isUnknown,
              }"
            />
            <span class="status-value">
              {{ isUnknown ? t('console.healthUnknown') : isOnline ? t('console.healthOnline') : t('console.healthOffline') }}
            </span>
          </div>
        </div>
      </div>
    </div>

    <!-- Executable path -->
    <div v-if="gatewayStatus?.executable_path" class="status-path">
      <Cpu :size="12" class="text-slate-500 shrink-0" />
      <span class="status-path-text">{{ gatewayStatus.executable_path }}</span>
    </div>

    <!-- Action buttons -->
    <div class="flex flex-wrap gap-2">
      <button
        type="button"
        class="action-btn action-btn--primary"
        :disabled="busy || isRunning"
        @click="startGateway"
      >
        <Play :size="14" />
        {{ t('console.startGateway') }}
      </button>
      <button
        type="button"
        class="action-btn action-btn--secondary"
        :disabled="busy || !isRunning"
        @click="stopGateway"
      >
        <Square :size="14" />
        {{ t('console.stopGateway') }}
      </button>
      <button
        type="button"
        class="action-btn action-btn--ghost"
        :disabled="loading"
        @click="refresh"
      >
        <Activity :size="14" />
        {{ t('console.refreshStatus') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.status-panel {
  @apply flex flex-col;
}

.status-card {
  background: var(--accent-bg-light);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 0.75rem 1rem;
}

.status-card-header {
  @apply flex items-center gap-2 mb-2;
}

.status-card-icon {
  @apply w-6 h-6 rounded-lg flex items-center justify-center;
}

.status-card-title {
  font-size: 0.75rem;
  font-weight: 500;
  color: var(--text-muted);
}

.status-card-body {
  @apply min-h-[2rem];
}

.status-dot {
  @apply w-2 h-2 rounded-full;
}

.status-dot--running {
  @apply bg-emerald-500 animate-pulse;
}

.status-dot--stopped {
  background: var(--text-muted);
}

.status-dot--online {
  @apply bg-emerald-500;
}

.status-dot--offline {
  @apply bg-rose-500;
}

.status-dot--unknown {
  background: var(--text-muted);
  @apply animate-pulse;
}

.status-value {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--text);
}

.status-detail {
  font-size: 0.75rem;
  color: var(--text-muted);
  word-break: break-all;
  margin-top: 0.25rem;
}

.status-path {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
  color: var(--text-muted);
  background: var(--accent-bg-light);
  border-radius: var(--radius-sm);
  padding: 0.5rem 0.75rem;
}

.status-path-text {
  @apply break-all font-mono;
}

.action-btn {
  @apply inline-flex items-center gap-2 px-3 py-2 text-sm rounded-lg transition-all;
}

.action-btn--primary {
  background: var(--accent);
  color: white;
}

.action-btn--primary:hover:not(:disabled) {
  filter: brightness(1.1);
}

.action-btn--secondary {
  background: transparent;
  border: 1px solid var(--line);
  color: var(--text);
}

.action-btn--secondary:hover:not(:disabled) {
  background: var(--nav-hover);
}

.action-btn--ghost {
  color: var(--text-muted);
}

.action-btn--ghost:hover:not(:disabled) {
  background: var(--accent-bg-light);
}

.action-btn:disabled {
  opacity: 0.6;
}
</style>