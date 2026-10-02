<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { Zap } from '@lucide/vue';
import TokenStatsPanel from './console/TokenStatsPanel.vue';

const { t } = useI18n();
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
