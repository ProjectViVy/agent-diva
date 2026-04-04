<script setup lang="ts">
import { computed } from 'vue';
import { Pencil, Trash2, Power, type LucideIcon } from 'lucide-vue-next';
import { PLATFORM_ICONS, PLATFORM_DISPLAY_NAMES } from './channel-icons';
import type { ChannelStatusSummary } from '../../api/desktop';

interface Channel {
  name: string;
  enabled: boolean;
  config?: Record<string, any>;
}

const props = defineProps<{
  channel: Channel;
  status?: ChannelStatusSummary;
}>();

const emit = defineEmits<{
  (e: 'toggle'): void;
  (e: 'edit'): void;
  (e: 'delete'): void;
}>();

const platform = computed(() => {
  // 尝试从 name 推断平台类型，或从 config 中获取
  const name = props.channel.name.toLowerCase();
  for (const key of Object.keys(PLATFORM_ICONS)) {
    if (name.includes(key)) return key;
  }
  // 默认返回第一个或 unknown
  return 'telegram';
});

const IconComponent = computed<LucideIcon>(() => {
  return PLATFORM_ICONS[platform.value] || PLATFORM_ICONS.telegram;
});

const displayName = computed(() => {
  return PLATFORM_DISPLAY_NAMES[platform.value] || props.channel.name;
});

const isReady = computed(() => {
  return props.status?.ready ?? false;
});

const isEnabled = computed(() => {
  return props.channel.enabled;
});

const missingFields = computed(() => {
  return props.status?.missing_fields ?? [];
});
</script>

<template>
  <div
    class="channel-card"
    :class="{
      enabled: isEnabled,
      ready: isReady,
      disabled: !isEnabled,
    }"
  >
    <div class="card-header">
      <div class="platform-logo">
        <IconComponent :size="24" />
      </div>
      <div
        class="status-badge"
        :class="isReady ? 'ready' : 'warning'"
      >
        {{ isReady ? $t('channels.activated') : $t('channels.needsConfig') }}
      </div>
    </div>

    <div class="card-body">
      <h3 class="channel-name">{{ displayName }}</h3>
      <p class="channel-status">
        {{ isEnabled ? $t('channels.enabled') : $t('channels.disabled') }}
      </p>
      <div v-if="!isReady && missingFields.length > 0" class="channel-meta">
        <span class="missing-label">{{ $t('channels.missing') }}:</span>
        <span class="missing-fields">{{ missingFields.slice(0, 2).join(', ') }}{{ missingFields.length > 2 ? '...' : '' }}</span>
      </div>
    </div>

    <div class="card-actions">
      <button
        class="action-btn"
        @click="emit('toggle')"
        :title="isEnabled ? $t('channels.disabled') : $t('channels.enabled')"
      >
        <Power :size="16" />
      </button>
      <button
        class="action-btn"
        @click="emit('edit')"
        :title="$t('settings.edit')"
      >
        <Pencil :size="16" />
      </button>
      <button
        class="action-btn danger"
        @click="emit('delete')"
        :title="$t('settings.delete')"
      >
        <Trash2 :size="16" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.channel-card {
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  padding: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
  transition: all 0.2s ease;
  min-width: 260px;
  max-width: 320px;
}

.channel-card:hover {
  border-color: var(--accent);
  box-shadow: 0 4px 12px var(--shadow);
  transform: translateY(-2px);
}

.channel-card.enabled.ready {
  border-left: 4px solid var(--success);
}

.channel-card.enabled:not(.ready) {
  border-left: 4px solid var(--warning);
}

.channel-card.disabled {
  opacity: 0.6;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.platform-logo {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-sm);
  background: var(--accent-bg-light);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
}

.status-badge {
  padding: 0.25rem 0.75rem;
  border-radius: 9999px;
  font-size: 0.75rem;
  font-weight: 600;
}

.status-badge.ready {
  background: var(--success);
  color: white;
}

.status-badge.warning {
  background: var(--warning);
  color: white;
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.channel-name {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
  margin: 0;
}

.channel-status {
  font-size: 0.875rem;
  color: var(--text-muted);
  margin: 0;
}

.channel-meta {
  display: flex;
  gap: 0.5rem;
  align-items: center;
  font-size: 0.75rem;
}

.missing-label {
  color: var(--text-muted);
  font-weight: 500;
}

.missing-fields {
  color: var(--danger);
}

.card-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
  margin-top: auto;
}

.action-btn {
  width: 32px;
  height: 32px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.action-btn:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

.action-btn.danger:hover {
  background: var(--danger);
  color: white;
}
</style>
