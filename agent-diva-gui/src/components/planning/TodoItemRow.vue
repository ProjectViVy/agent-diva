<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  Loader2,
  Circle,
  Lock,
  CheckCircle2,
  ExternalLink,
  RotateCcw,
  Trash2,
  Loader2 as ActionLoader,
} from 'lucide-vue-next';
import type { TodoDetail } from '../../api/planning';

const { t } = useI18n();

const props = defineProps<{
  todo: TodoDetail;
  detailed?: boolean;
  canDelete?: boolean;
  canRestore?: boolean;
  busy?: boolean;
}>();

const emit = defineEmits<{ (event: 'delete'): void; (event: 'restore'): void }>();

const todoStatus = computed(() => props.todo.status.toLowerCase());
const isInProgress = computed(() => todoStatus.value === 'in_progress' || todoStatus.value === 'inprogress');
const isPending = computed(() => todoStatus.value === 'pending');
const isBlocked = computed(() => todoStatus.value === 'blocked');
const isCompleted = computed(() => todoStatus.value === 'completed');
const isCanceled = computed(() => todoStatus.value === 'canceled');

const isHighPriority = computed(() => props.todo.priority === 'high');
</script>

<template>
  <div
    class="todo-row"
    :class="{
      'todo-row--completed': isCompleted,
      'todo-row--blocked': isBlocked,
      'todo-row--canceled': isCanceled,
    }"
  >
    <!-- Status icon -->
    <div class="todo-status-icon">
      <Loader2
        v-if="isInProgress"
        :size="14"
        class="animate-spin"
        style="color: var(--warning)"
      />
      <Circle
        v-else-if="isPending"
        :size="14"
        style="color: var(--text-muted)"
      />
      <Lock
        v-else-if="isBlocked"
        :size="14"
        style="color: var(--danger)"
      />
      <Trash2
        v-else-if="isCanceled"
        :size="14"
        style="color: var(--text-muted)"
      />
      <CheckCircle2
        v-else-if="isCompleted"
        :size="14"
        style="color: var(--success)"
      />
    </div>

    <!-- Title -->
    <span
      class="todo-title"
      :class="{ 'todo-title--done': isCompleted }"
    >
      {{ todo.title }}
    </span>

    <span v-if="detailed && todo.detail" class="todo-detail">{{ todo.detail }}</span>

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

    <span v-if="detailed && todo.plan_step_id" class="todo-step-ref">{{ todo.plan_step_id }}</span>

    <ActionLoader v-if="busy" :size="13" class="todo-action animate-spin" />
    <button v-else-if="canRestore" type="button" class="todo-action" title="复原任务" aria-label="复原任务" @click="emit('restore')">
      <RotateCcw :size="13" />
    </button>
    <button v-else-if="canDelete" type="button" class="todo-action todo-action-danger" title="删除任务" aria-label="删除任务" @click="emit('delete')">
      <Trash2 :size="13" />
    </button>
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
  background: var(--accent-bg-light);
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
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.todo-title--done {
  text-decoration: line-through;
  color: var(--text-muted);
}

.todo-detail {
  flex: 1 1 100%;
  min-width: 0;
  margin-left: 26px;
  color: var(--text-muted);
  font-size: 0.75rem;
  line-height: 1.4;
}

.todo-step-ref {
  color: var(--text-muted);
  font-size: 0.65rem;
  white-space: nowrap;
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
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 15%, transparent);
}

.todo-priority-badge--low {
  color: var(--text-muted);
  background: color-mix(in srgb, var(--line) 10%, transparent);
}

/* Evidence link */
.todo-evidence {
  display: flex;
  align-items: center;
  color: var(--accent);
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
  color: var(--danger);
  opacity: 0.8;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 160px;
  flex-shrink: 0;
}
.todo-action { display: inline-flex; align-items: center; justify-content: center; flex: 0 0 auto; padding: 3px; border: 0; border-radius: 5px; color: var(--text-muted); background: transparent; cursor: pointer; }
.todo-action:hover { color: var(--accent); background: var(--accent-bg-light); }
.todo-action-danger:hover { color: var(--danger); }
</style>
