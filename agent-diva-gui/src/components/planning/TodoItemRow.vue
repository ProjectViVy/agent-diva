<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Loader2,
  Circle,
  Lock,
  CheckCircle2,
  ExternalLink,
} from 'lucide-vue-next';
import type { TodoDetail } from '../../api/planning';

const { t } = useI18n();

const props = defineProps<{
  todo: TodoDetail;
}>();

const isInProgress = computed(() => props.todo.status === 'in_progress');
const isPending = computed(() => props.todo.status === 'pending');
const isBlocked = computed(() => props.todo.status === 'blocked');
const isCompleted = computed(() => props.todo.status === 'completed');

const isHighPriority = computed(() => props.todo.priority === 'high');
</script>

<template>
  <div
    class="todo-row"
    :class="{
      'todo-row--completed': isCompleted,
      'todo-row--blocked': isBlocked,
    }"
  >
    <!-- Status icon -->
    <div class="todo-status-icon">
      <Loader2
        v-if="isInProgress"
        :size="14"
        class="animate-spin"
        style="color: var(--warning, #ffa726)"
      />
      <Circle
        v-else-if="isPending"
        :size="14"
        style="color: var(--text-muted, rgba(240, 230, 239, 0.4))"
      />
      <Lock
        v-else-if="isBlocked"
        :size="14"
        style="color: var(--danger, #ef5350)"
      />
      <CheckCircle2
        v-else-if="isCompleted"
        :size="14"
        style="color: var(--success, #66bb6a)"
      />
    </div>

    <!-- Title -->
    <span
      class="todo-title"
      :class="{ 'todo-title--done': isCompleted }"
    >
      {{ todo.title }}
    </span>

    <!-- Priority badge -->
    <span
      v-if="isHighPriority"
      class="todo-priority-badge todo-priority-badge--high"
    >
      {{ t('planning.highPriority') }}
    </span>
    <span
      v-else-if="todo.priority === 'low'"
      class="todo-priority-badge todo-priority-badge--low"
    >
      {{ t('planning.lowPriority') }}
    </span>

    <!-- Evidence link (completed) -->
    <a
      v-if="isCompleted && todo.evidence_ref"
      :href="todo.evidence_ref"
      target="_blank"
      rel="noopener noreferrer"
      class="todo-evidence"
      :title="t('planning.evidence')"
    >
      <ExternalLink :size="12" />
    </a>

    <!-- Block reason (blocked) -->
    <span
      v-if="isBlocked && todo.block_reason"
      class="todo-block-reason"
      :title="todo.block_reason"
    >
      🔒 {{ todo.block_reason }}
    </span>
  </div>
</template>

<style scoped>
.todo-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.4rem 0.5rem;
  border-radius: 6px;
  transition: background 0.15s ease;
}

.todo-row:hover {
  background: rgba(236, 64, 122, 0.06);
}

.todo-row--completed {
  opacity: 0.65;
}

.todo-row--blocked {
  opacity: 0.8;
}

.todo-status-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  min-width: 18px;
  height: 18px;
  flex-shrink: 0;
}

.todo-title {
  flex: 1;
  font-size: 0.85rem;
  color: var(--text, #f0e6ef);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.todo-title--done {
  text-decoration: line-through;
  color: var(--text-muted, rgba(240, 230, 239, 0.4));
}

/* Priority badges */
.todo-priority-badge {
  font-size: 0.65rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  padding: 0.1rem 0.4rem;
  border-radius: 9999px;
  flex-shrink: 0;
}

.todo-priority-badge--high {
  color: var(--accent, #ec407a);
  background: rgba(236, 64, 122, 0.15);
}

.todo-priority-badge--low {
  color: var(--text-muted, rgba(240, 230, 239, 0.5));
  background: rgba(255, 255, 255, 0.05);
}

/* Evidence link */
.todo-evidence {
  display: flex;
  align-items: center;
  color: var(--accent, #ec407a);
  opacity: 0.7;
  transition: opacity 0.15s ease;
  flex-shrink: 0;
}

.todo-evidence:hover {
  opacity: 1;
}

/* Block reason */
.todo-block-reason {
  font-size: 0.7rem;
  color: var(--danger, #ef5350);
  opacity: 0.8;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 160px;
  flex-shrink: 0;
}
</style>
