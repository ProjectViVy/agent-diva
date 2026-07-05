<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { AlertCircle } from 'lucide-vue-next';

const { t } = useI18n();

interface Props {
  title: string;
  message?: string;
  onRetry: () => void;
}

defineProps<Props>();
</script>

<template>
  <div class="persona-memory-error-state" role="alert">
    <AlertCircle :size="48" class="error-icon" />
    <strong class="error-title">{{ title }}</strong>
    <p v-if="message" class="error-message">{{ message }}</p>
    <button
      type="button"
      class="error-retry"
      @click="onRetry"
    >
      {{ t('laputa.retry') }}
    </button>
  </div>
</template>

<style scoped>
.persona-memory-error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 24px;
  text-align: center;
  gap: 10px;
}

.error-icon {
  color: var(--danger);
}

.error-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
}

.error-message {
  font-size: 0.875rem;
  color: var(--text-muted);
  margin: 0;
  max-width: 360px;
}

.error-retry {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  margin-top: 8px;
  padding: 6px 14px;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.error-retry:hover {
  background: var(--accent-bg-light);
  border-color: var(--accent-border);
  color: var(--accent);
}
</style>
