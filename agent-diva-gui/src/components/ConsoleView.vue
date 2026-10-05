<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Activity, ScrollText, Zap } from '@lucide/vue';
import TokenStatsPanel from './console/TokenStatsPanel.vue';
import TrajectoryPanel from './console/TrajectoryPanel.vue';
import DiagnosticsPanel from './console/DiagnosticsPanel.vue';
import { vivyChat } from '../state/chat-instance';
import type { TrajectoryView } from '../state/vivy-trajectory';

const { t } = useI18n();

const trajView = ref<TrajectoryView | null>(null);
const trajError = ref<string | null>(null);
const connState = ref<string>('disconnected');

function sync(): void {
  trajView.value = vivyChat.trajectory();
  trajError.value = vivyChat.trajectoryError();
  connState.value = vivyChat.projection.connection;
}

let detach: (() => void) | null = null;
onMounted(() => {
  detach = vivyChat.subscribe(sync);
  sync();
});
onBeforeUnmount(() => {
  detach?.();
  detach = null;
});
</script>

<template>
  <div class="console-view h-full min-h-0 overflow-y-auto">
    <div class="mx-auto max-w-5xl space-y-6 p-6">
      <section class="console-section">
        <div class="flex items-center gap-3 mb-4">
          <div class="console-section-icon console-section-icon--purple">
            <Zap :size="20" />
          </div>
          <div>
            <h3 class="console-section-title">{{ t('tokenStats.title', 'Token Statistics') }}</h3>
            <p class="console-section-desc">{{ t('tokenStats.subtitle') }}</p>
          </div>
        </div>

        <TokenStatsPanel />
      </section>

      <section class="console-section">
        <div class="flex items-center gap-3 mb-4">
          <div class="console-section-icon console-section-icon--blue">
            <Activity :size="20" />
          </div>
          <div>
            <h3 class="console-section-title">{{ t('trajectory.title', 'Session Activity') }}</h3>
            <p class="console-section-desc">{{ t('trajectory.desc', 'Runs, model calls, and records for the selected session') }}</p>
          </div>
        </div>

        <TrajectoryPanel :view="trajView" :error="trajError" :connection-state="connState" />
      </section>

      <section class="console-section">
        <div class="flex items-center gap-3 mb-4">
          <div class="console-section-icon console-section-icon--green">
            <ScrollText :size="20" />
          </div>
          <div>
            <h3 class="console-section-title">{{ t('diagnostics.title', 'Diagnostics') }}</h3>
            <p class="console-section-desc">{{ t('diagnostics.desc', 'Bounded runtime and GUI log tails') }}</p>
          </div>
        </div>

        <DiagnosticsPanel />
      </section>
    </div>
  </div>
</template>

<style scoped>
.console-section {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 1.5rem;
  margin-bottom: 1.5rem;
  box-shadow: var(--shadow-md);
}

.console-section-icon {
  width: 2.5rem;
  height: 2.5rem;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
}

.console-section-icon--purple {
  background: var(--info-soft);
  color: var(--info);
}

.console-section-icon--blue {
  background: var(--info-soft);
  color: var(--info);
}

.console-section-icon--green {
  background: var(--success-soft);
  color: var(--success);
}

.console-section-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--foreground);
  margin-bottom: 0.25rem;
}

.console-section-desc {
  font-size: 0.875rem;
  color: var(--muted-foreground);
}
</style>
