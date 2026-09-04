<script setup lang="ts">
import { computed } from 'vue';
import { CircleAlert, LoaderCircle, Plus, MessageSquarePlus } from '@lucide/vue';
import ChannelCard from './ChannelCard.vue';
import type { ChannelStatusSummary } from '../../api/desktop';

interface Channel {
  name: string;
  enabled: boolean;
  config?: Record<string, any>;
}

const props = defineProps<{
  channels: Record<string, Record<string, any>>;
  statuses: ChannelStatusSummary[];
  loading?: boolean;
  canAdd?: boolean;
  busyChannels?: string[];
  errors?: Record<string, string>;
  error?: string | null;
}>();

const emit = defineEmits<{
  (e: 'add'): void;
  (e: 'edit', name: string): void;
  (e: 'delete', name: string): void;
  (e: 'toggle', name: string): void;
}>();

const statusMap = computed(() => new Map(props.statuses.map((s) => [s.name, s])));
const busyChannelSet = computed(() => new Set(props.busyChannels ?? []));
const canAddAction = computed(() => props.canAdd ?? true);

const channelList = computed(() =>
  Object.entries(props.channels)
    .map(([name, raw]) => ({
      name,
      channel: { name, enabled: Boolean(raw?.enabled), config: raw } as Channel,
        status: statusMap.value.get(name),
        error: props.errors?.[name],
      }))
);
</script>

<template>
  <div class="channel-card-view">
    <!-- Empty State -->
    <div v-if="loading && channelList.length === 0" class="channel-loading-state" role="status">
      <LoaderCircle :size="32" class="animate-spin" />
      <span>{{ $t('channels.loading') }}</span>
    </div>

    <div v-else-if="error && channelList.length === 0" class="channel-error-state" role="alert">
      <CircleAlert :size="32" />
      <span>{{ error }}</span>
    </div>

    <div v-else-if="channelList.length === 0" class="channel-empty-state">
      <div class="empty-icon">
        <MessageSquarePlus :size="80" />
      </div>
      <h3>{{ $t('channels.noChannels') }}</h3>
      <p>{{ $t('channels.noChannelsHint') }}</p>
      <div v-if="canAddAction" class="empty-actions">
        <button class="btn-primary" @click="emit('add')">
          <Plus :size="16" />
          {{ $t('channels.addChannel') }}
        </button>
      </div>
    </div>

    <!-- Card Grid -->
    <div v-else class="channel-card-grid">
      <ChannelCard
        v-for="{ name, channel, status, error: channelError } in channelList"
        :key="name"
        :channel="channel"
        :status="status"
        :busy="busyChannelSet.has(name)"
        :error="channelError"
        @toggle="emit('toggle', name)"
        @edit="emit('edit', name)"
        @delete="emit('delete', name)"
      />
    </div>
  </div>
</template>

<style scoped>
.channel-card-view {
  width: 100%;
  height: 100%;
  overflow-y: auto;
}

.channel-loading-state,
.channel-error-state {
  display: flex;
  min-height: 14rem;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 4rem 2rem;
  text-align: center;
}

.channel-loading-state {
  color: var(--text-muted);
}

.channel-error-state {
  color: var(--danger);
}

.channel-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 4rem 2rem;
  text-align: center;
}

.empty-icon {
  color: var(--text-muted);
  opacity: 0.3;
  margin-bottom: 1.5rem;
}

.channel-empty-state h3 {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.channel-empty-state p {
  font-size: 0.875rem;
  color: var(--text-muted);
  margin-bottom: 1.5rem;
}

.empty-actions {
  display: flex;
  gap: 0.75rem;
}

.btn-primary,
.btn-secondary {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.625rem 1.25rem;
  border-radius: var(--radius-sm);
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s ease;
}

.btn-primary {
  background: var(--accent);
  color: white;
  border: none;
}

.btn-primary:hover {
  filter: brightness(1.1);
}

.btn-secondary {
  background: var(--panel);
  color: var(--text);
  border: 1px solid var(--line);
}

.btn-secondary:hover {
  background: var(--accent-bg-light);
}

.channel-card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1rem;
  padding: 1.5rem;
}
</style>
