<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Activity, Zap } from '@lucide/vue';
import TokenStatsPanel from './console/TokenStatsPanel.vue';
import TrajectoryPanel from './console/TrajectoryPanel.vue';
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
            <p class="console-section-desc">{{ t('tokenStats.desc', 'Monitor AI model usage and costs') }}</p>
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
    </div>
  </div>
</template>

<style scoped>
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

.console-section-icon--purple {
  background: rgba(139, 92, 246, 0.15);
  color: #8b5cf6;
}

.console-section-icon--blue {
  background: rgba(59, 130, 246, 0.15);
  color: #3b82f6;
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
</style>
