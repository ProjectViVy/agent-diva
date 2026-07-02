<script setup lang="ts">
import { ref, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ChevronDown, ChevronRight } from 'lucide-vue-next';
import TodoItemRow from './TodoItemRow.vue';
import type { TodoDetail } from '../../api/planning';

const { t } = useI18n();

const props = defineProps<{
  todos: TodoDetail[];
}>();

// --- Group by status ---
const inProgressTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status === 'in_progress'),
);

const pendingTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status === 'pending'),
);

const blockedTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status === 'blocked'),
);

const completedTodos = computed(() =>
  props.todos.filter((t: TodoDetail) => t.status === 'completed'),
);

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
    color: 'var(--warning, #ffa726)',
  },
  {
    key: 'pending',
    label: t('planning.pending'),
    todos: pendingTodos.value,
    color: 'var(--text-muted, rgba(240, 230, 239, 0.55))',
  },
  {
    key: 'blocked',
    label: t('planning.blocked'),
    todos: blockedTodos.value,
    color: 'var(--danger, #ef5350)',
  },
  {
    key: 'completed',
    label: t('planning.completed'),
    todos: completedTodos.value,
    color: 'var(--success, #66bb6a)',
  },
]);

function toggleGroup(key: string) {
  collapsedGroups.value[key] = !collapsedGroups.value[key];
}
</script>

<template>
  <div class="todo-list-panel">
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
          />
        </div>
      </div>
    </template>

    <!-- Empty state -->
    <div v-if="todos.length === 0" class="panel-empty">
      <span class="text-sm text-gray-500">—</span>
    </div>
  </div>
</template>

<style scoped>
.todo-list-panel {
  background: var(--bg-panel, rgba(30, 20, 28, 0.95));
  border: 1px solid var(--line, rgba(255, 255, 255, 0.08));
  border-radius: 12px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  transition: border-color 0.2s ease;
}

.todo-list-panel:hover {
  border-color: var(--accent-border, rgba(236, 64, 122, 0.3));
}

/* Progress bar */
.panel-progress {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid var(--line, rgba(255, 255, 255, 0.06));
}

.panel-progress-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.panel-progress-label {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--text-muted, rgba(240, 230, 239, 0.55));
}

.panel-progress-count {
  font-size: 0.8rem;
  font-weight: 700;
  color: var(--accent, #ec407a);
}

.panel-progress-bar {
  height: 6px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  overflow: hidden;
}

.panel-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent, #ec407a), var(--accent-light, #f48fb1));
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
  color: var(--text, #f0e6ef);
}

.group-header:hover {
  background: rgba(236, 64, 122, 0.06);
}

.group-header-left {
  display: flex;
  align-items: center;
  gap: 0.375rem;
}

.group-toggle {
  color: var(--text-muted, rgba(240, 230, 239, 0.4));
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
  color: var(--text-muted, rgba(240, 230, 239, 0.4));
  background: rgba(255, 255, 255, 0.05);
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
