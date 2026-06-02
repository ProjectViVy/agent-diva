<script setup lang="ts">
import { computed } from 'vue';
import { Plus, MessageSquarePlus } from 'lucide-vue-next';
import ChannelCard from './ChannelCard.vue';
import type { ChannelStatusSummary } from '../../api/desktop';

interface Channel {
  name: string;
  enabled: boolean;
  config?: Record<string, any>;
}

const props = defineProps<{
  channels: Record<string, Channel>;
  statuses: ChannelStatusSummary[];
  loading?: boolean;
}>();

const emit = defineEmits<{
  (e: 'add'): void;
  (e: 'edit', name: string): void;
  (e: 'delete', name: string): void;
  (e: 'toggle', name: string): void;
}>();

const statusMap = computed(() => new Map(props.statuses.map((s) => [s.name, s])));

const channelList = computed(() =>
  Object.entries(props.channels).map(([name, channel]) => ({
    name,
    channel,
    status: statusMap.value.get(name),
  }))
);
</script>

<template>
  <div class="channel-card-view">
    <!-- Empty State -->
    <div v-if="Object.keys(channels).length === 0" class="channel-empty-state">
      <div class="empty-icon">
        <MessageSquarePlus :size="80" />
      </div>
      <h3>{{ $t('channels.noChannels') }}</h3>
      <p>{{ $t('channels.noChannelsHint') }}</p>
      <div class="empty-actions">
        <button class="btn-primary" @click="emit('add')">
          <Plus :size="16" />
          {{ $t('channels.addChannel') }}
        </button>
      </div>
    </div>

    <!-- Card Grid -->
    <div v-else class="channel-card-grid">
      <ChannelCard
        v-for="{ name, channel, status } in channelList"
        :key="name"
        :channel="channel"
        :status="status"
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
