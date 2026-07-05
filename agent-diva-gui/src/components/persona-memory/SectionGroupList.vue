<script setup lang="ts">
import type { LaputaSectionName } from '../../api/desktop';

interface SnapshotSection {
  status: 'owned' | 'tbd';
  last_modified?: string | null;
}

interface Props {
  snapshot: { sections: Record<string, SnapshotSection> } | null;
  selectedSection: LaputaSectionName;
}

defineProps<Props>();

const emit = defineEmits<{
  (e: 'select', name: LaputaSectionName): void;
}>();

function select(name: LaputaSectionName) {
  emit('select', name);
}
</script>

<template>
  <div class="section-group-list">
    <div
      v-for="(_, name) in snapshot?.sections"
      :key="name"
      class="section-group-item"
      :class="{ selected: name === selectedSection }"
      @click="select(name as LaputaSectionName)"
    >
      {{ name }}
    </div>
  </div>
</template>

<style scoped>
.section-group-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px;
}

.section-group-item {
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  color: var(--text);
  font-size: 13px;
  transition: all 0.15s;
}

.section-group-item:hover {
  background: var(--accent-bg-light);
}

.section-group-item.selected {
  background: var(--accent-bg-light);
  color: var(--accent);
  font-weight: 500;
}
</style>
