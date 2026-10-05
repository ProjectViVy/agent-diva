<script setup lang="ts">
import { ref, computed } from 'vue';
import { Server, Check, LoaderCircle, PlugZap, Trash2, Edit3 } from '@lucide/vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

interface ProviderCardProps {
  name: string;
  displayName: string;
  icon?: string;
  status: 'ready' | 'missingConfig' | 'active';
  currentModel?: string;
  apiBase?: string;
  apiKey?: string;
  isCustom?: boolean;
}

const props = withDefaults(defineProps<ProviderCardProps>(), {
  icon: '',
  currentModel: '',
  apiBase: '',
  apiKey: '',
  isCustom: false,
});

/** Mask API key to show only last 4 chars: ****abcd */
const maskedApiKey = computed(() => {
  const key = props.apiKey;
  if (!key) return '';
  if (key.length <= 4) return '****';
  return '****' + key.slice(-4);
});

const emit = defineEmits<{
  (e: 'edit'): void;
  (e: 'delete'): void;
  (e: 'test'): void;
}>();

const isHovered = ref(false);
const isTesting = ref(false);

const statusConfig = computed(() => {
  const config = {
    ready: {
      label: t('providers.ready'),
      class: 'text-success',
      icon: Check
    },
    missingConfig: {
      label: t('providers.missingConfig'),
      class: 'text-warning',
      icon: null
    },
    active: {
      label: t('providers.currentTag'),
      class: 'text-success',
      icon: Check
    },
  };
  return config[props.status];
});
</script>

<template>
  <div
    class="ui-card providers-card"
    :class="{ 'is-active': status === 'active' }"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
  >
    <div class="providers-card-icon">
      <Server :size="24" />
    </div>

    <div class="providers-card-body">
      <h3 class="providers-card-title">{{ displayName }}</h3>
      <div class="providers-card-status" :class="statusConfig.class">
        <component :is="statusConfig.icon" v-if="statusConfig.icon" :size="12" />
        <span>{{ statusConfig.label }}</span>
      </div>
      <p v-if="currentModel" class="providers-card-model">
        {{ currentModel }}
      </p>
      <p v-else-if="apiBase" class="providers-card-api-base">
        {{ apiBase }}
      </p>
      <p v-if="maskedApiKey" class="providers-card-api-base">
        {{ maskedApiKey }}
      </p>
    </div>

    <div
      class="providers-card-actions"
      :class="{ visible: isHovered || isTesting }"
    >
      <button
        class="ui-button ui-button--ghost ui-button--compact ui-button--icon providers-card-action-btn"
        :title="t('providers.testConnection')"
        @click.stop="emit('test')"
        :disabled="isTesting"
      >
        <LoaderCircle v-if="isTesting" :size="14" class="animate-spin" />
        <PlugZap v-else :size="14" />
      </button>
      <button
        class="ui-button ui-button--ghost ui-button--compact ui-button--icon providers-card-action-btn"
        :title="t('providers.edit')"
        @click.stop="emit('edit')"
      >
        <Edit3 :size="14" />
      </button>
      <button
        v-if="isCustom"
        class="ui-button ui-button--danger-ghost ui-button--compact ui-button--icon providers-card-action-btn"
        :title="t('providers.deleteProvider')"
        @click.stop="emit('delete')"
      >
        <Trash2 :size="14" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.providers-card {
  position: relative;
  padding: 1.5rem;
  transition: all 0.15s ease;
  cursor: pointer;
  overflow: hidden;
}

.providers-card:hover {
  border-color: var(--border-strong);
  box-shadow: var(--shadow-sm);
}

.providers-card.is-active {
  border-color: var(--primary);
  box-shadow: var(--shadow-sm);
}

.providers-card-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-md);
  background: var(--accent);
  color: var(--primary);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 1rem;
  transition: transform 0.15s ease;
}

.providers-card:hover .providers-card-icon {
  transform: scale(1.1);
}

.providers-card-body {
  margin-bottom: 1rem;
}

.providers-card-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--foreground);
  margin-bottom: 0.5rem;
}

.providers-card-status {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.75rem;
  font-weight: 500;
  margin-bottom: 0.25rem;
}

.providers-card-model,
.providers-card-api-base {
  font-size: 0.75rem;
  color: var(--muted-foreground);
  font-family: monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.providers-card-actions {
  position: absolute;
  top: 1rem;
  right: 1rem;
  display: flex;
  gap: 0.5rem;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.providers-card-actions.visible {
  opacity: 1;
}

.providers-card-action-btn {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
