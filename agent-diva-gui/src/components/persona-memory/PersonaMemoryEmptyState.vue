<script setup lang="ts">
import type { Component } from 'vue';

interface Props {
  icon: Component;
  title: string;
  description: string;
}

defineProps<Props>();
</script>

<template>
  <div class="persona-memory-empty-state" role="status">
    <component :is="icon" :size="48" class="empty-icon" />
    <strong class="empty-title">{{ title }}</strong>
    <p class="empty-description">{{ description }}</p>
  </div>
</template>

<style scoped>
.persona-memory-empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  flex: 1;
  padding: 24px;
  text-align: center;
  gap: 8px;
}

.empty-icon {
  color: var(--text-muted);
  opacity: 0.5;
  margin-bottom: 4px;
}

.empty-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-muted);
}

.empty-description {
  font-size: 0.875rem;
  color: var(--text-muted);
  margin: 0;
  max-width: 360px;
}
</style>
