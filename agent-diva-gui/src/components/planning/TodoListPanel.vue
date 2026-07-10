<script setup lang="ts">
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from 'vue-i18n';
import { ChevronDown, ChevronRight, List, ListChecks } from 'lucide-vue-next';
import TodoItemRow from './TodoItemRow.vue';
import type { TodoDetail } from '../../api/planning';

const { t } = useI18n();

const props = defineProps<{
  todos: TodoDetail[];
  planTitle?: string;
  planPhase?: string;
  planId: string;
}>();

const emit = defineEmits<{ (event: 'changed'): void }>();

const detailedMode = ref(false);

// --- Group by status ---
const inProgressTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status.toLowerCase() === 'inprogress' || t.status.toLowerCase() === 'in_progress'),
);

const pendingTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status.toLowerCase() === 'pending'),
);

const blockedTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status.toLowerCase() === 'blocked'),
);

const completedTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status.toLowerCase() === 'completed'),
);
const canceledTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status.toLowerCase() === 'canceled'),
);
const busyTodoId = ref<string | null>(null);

// --- Progress ---
const totalCount = computed(() => props.todos.length);
const completedCount = computed(() => completedTodos.value.length);
const progressPercent = computed(() => {
  if (totalCount.value === 0) return 0;
  return Math.round((completedCount.value / totalCount.value) * 100);
});

// --- Collapse state per group ---
const collapsedGroups = ref<Record<string, boolean>>({
  in_progress: false,
  pending: false,
  blocked: false,
  completed: true, // collapsed by default
  canceled: true,
});

interface TodoGroup {
  key: string;
  label: string;
  todos: TodoDetail[];
  color: string;
}

const groups = computed<TodoGroup[]>(() => [
  {
    key: 'in_progress',
    label: t('planning.inProgress'),
    todos: inProgressTodos.value,
    color: 'var(--warning)',
  },
  {
    key: 'pending',
    label: t('planning.pending'),
    todos: pendingTodos.value,
    color: 'var(--text-muted)',
  },
  {
    key: 'blocked',
    label: t('planning.blocked'),
    todos: blockedTodos.value,
    color: 'var(--danger)',
  },
  {
    key: 'completed',
    label: t('planning.completed'),
    todos: completedTodos.value,
    color: 'var(--success)',
  },
  {
    key: 'canceled',
    label: '已删除',
    todos: canceledTodos.value,
    color: 'var(--text-muted)',
  },
]);

function toggleGroup(key: string) {
  collapsedGroups.value[key] = !collapsedGroups.value[key];
}

async function changeTodoState(todo: TodoDetail, action: 'delete' | 'restore') {
  if (busyTodoId.value) return;
  busyTodoId.value = todo.id;
  try {
    await invoke(action === 'delete' ? 'delete_plan_todo' : 'restore_plan_todo', {
      planId: props.planId,
      todoId: todo.id,
    });
    emit('changed');
  } catch (error) {
    console.warn(`[TodoListPanel] Failed to ${action} todo:`, error);
  } finally {
    busyTodoId.value = null;
  }
}
</script>

<template>
  <div class="todo-list-panel">
    <div v-if="planTitle" class="panel-title-row">
      <div class="panel-title-wrap">
        <ListChecks :size="16" class="panel-title-icon" />
        <div>
          <div class="panel-title">{{ planTitle }}</div>
          <div v-if="planPhase" class="panel-phase">{{ planPhase }}</div>
        </div>
      </div>
      <button class="detail-toggle" type="button" @click="detailedMode = !detailedMode">
        <List v-if="!detailedMode" :size="14" />
        <ListChecks v-else :size="14" />
        {{ detailedMode ? '简洁' : '详细' }}
      </button>
    </div>
    <!-- Progress bar -->
    <div class="panel-progress">
      <div class="panel-progress-header">
        <span class="panel-progress-label">{{ t('planning.progress') }}</span>
        <span class="panel-progress-count">{{ completedCount }}/{{ totalCount }}</span>
      </div>
      <div class="panel-progress-bar">
        <div
          class="panel-progress-fill"
          :style="{ width: `${progressPercent}%` }"
        />
      </div>
    </div>

    <!-- Groups -->
    <template v-for="group in groups" :key="group.key">
      <div v-if="group.todos.length > 0" class="todo-group">
        <!-- Group header (clickable to collapse) -->
        <button class="group-header" @click="toggleGroup(group.key)">
          <div class="group-header-left">
            <span class="group-toggle">
              <ChevronDown v-if="!collapsedGroups[group.key]" :size="14" />
              <ChevronRight v-else :size="14" />
            </span>
            <span class="group-label" :style="{ color: group.color }">
              {{ group.label }}
            </span>
            <span class="group-count">{{ group.todos.length }}</span>
          </div>
        </button>

        <!-- Group items -->
        <div v-show="!collapsedGroups[group.key]" class="group-items">
          <TodoItemRow
            v-for="todo in group.todos"
            :key="todo.id"
            :todo="todo"
            :detailed="detailedMode"
            :can-delete="group.key !== 'canceled'"
            :can-restore="group.key === 'canceled'"
            :busy="busyTodoId === todo.id"
            @delete="changeTodoState(todo, 'delete')"
            @restore="changeTodoState(todo, 'restore')"
          />
        </div>
      </div>
    </template>

    <!-- Empty state -->
    <div v-if="todos.length === 0" class="panel-empty">
      <span class="text-sm" style="color: var(--text-muted)">—</span>
    </div>
  </div>
</template>

<style scoped>
.todo-list-panel {
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  transition: border-color 0.2s ease;
}

.todo-list-panel:hover {
  border-color: var(--accent-border);
}

.panel-title-row {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  padding-bottom: 0.75rem;
  border-bottom: 1px solid var(--line);
}

.panel-title-wrap { display: flex; align-items: flex-start; gap: 0.5rem; min-width: 0; }
.panel-title-icon { flex: 0 0 auto; color: var(--accent); }
.panel-title { overflow: hidden; color: var(--text); font-size: 0.9rem; font-weight: 700; text-overflow: ellipsis; white-space: nowrap; }
.panel-phase { margin-top: 0.15rem; color: var(--text-muted); font-size: 0.7rem; }
.detail-toggle { display: inline-flex; align-items: center; gap: 0.25rem; flex: 0 0 auto; padding: 0.25rem 0.45rem; border: 1px solid var(--line); border-radius: 6px; color: var(--accent); background: var(--panel); font-size: 0.7rem; cursor: pointer; }
.detail-toggle:hover { background: var(--accent-bg-light); }

/* Progress bar */
.panel-progress {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--line);
}

.panel-progress-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-progress-label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-muted);
}

.panel-progress-count {
  font-size: 0.8rem;
  font-weight: 700;
  color: var(--accent);
}

.panel-progress-bar {
  height: 6px;
  background: color-mix(in srgb, var(--line) 15%, transparent);
  border-radius: 3px;
  overflow: hidden;
}

.panel-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--accent-light));
  border-radius: 3px;
  transition: width 0.4s ease;
}

/* Groups */
.todo-group {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.375rem 0.5rem;
  background: transparent;
  border: none;
  cursor: pointer;
  border-radius: 6px;
  transition: background 0.15s ease;
  color: var(--text);
}

.group-header:hover {
  background: color-mix(in srgb, var(--accent) 6%, transparent);
}

.group-header-left {
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

.group-toggle {
  color: var(--text-muted);
  display: flex;
  align-items: center;
}

.group-label {
  font-size: 0.8rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.group-count {
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--text-muted);
  background: color-mix(in srgb, var(--line) 12%, transparent);
  padding: 0.1rem 0.4rem;
  border-radius: 9999px;
}

.group-items {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  padding-left: 0.5rem;
}

.panel-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 1.5rem 0;
}
</style>
