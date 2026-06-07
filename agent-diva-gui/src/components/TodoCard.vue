<script setup lang="ts">
import { ref, computed, watch, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { Check, ChevronDown, ChevronUp } from 'lucide-vue-next'
import type { UiCard, TodoItem } from '../../api/desktop'

const props = defineProps<{
  card: UiCard
}>()

const emit = defineEmits<{
  (e: 'check', payload: { id: string; item_id: string; status: 'pending' | 'done' }): void
}>()

const { t } = useI18n()

const collapsed = ref(false)
let collapseTimer: ReturnType<typeof setTimeout> | null = null

const items = computed<TodoItem[]>(() => props.card.todo_items ?? [])

const doneCount = computed(() => items.value.filter(i => i.status === 'done').length)
const totalCount = computed(() => items.value.length)
const allDone = computed(() => totalCount.value > 0 && doneCount.value === totalCount.value)
const hasPending = computed(() => doneCount.value < totalCount.value)

// Auto-collapse 3s after all items are done
watch(allDone, (val) => {
  if (collapseTimer) {
    clearTimeout(collapseTimer)
    collapseTimer = null
  }
  if (val) {
    collapseTimer = setTimeout(() => {
      collapsed.value = true
    }, 3000)
  }
})

onUnmounted(() => {
  if (collapseTimer) {
    clearTimeout(collapseTimer)
  }
})

function toggleItem(item: TodoItem) {
  const newStatus: 'pending' | 'done' = item.status === 'done' ? 'pending' : 'done'
  emit('check', { id: props.card.id, item_id: item.id, status: newStatus })
}

function markAllDone() {
  for (const item of items.value) {
    if (item.status === 'pending') {
      emit('check', { id: props.card.id, item_id: item.id, status: 'done' })
    }
  }
}

function toggleCollapse() {
  collapsed.value = !collapsed.value
}

function formatTime(iso?: string): string {
  if (!iso) return ''
  try {
    const d = new Date(iso)
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  } catch {
    return ''
  }
}
</script>

<template>
  <div class="todo-card" :class="{ 'all-done': allDone }">
    <!-- Header -->
    <div class="todo-header" @click="toggleCollapse">
      <div class="todo-title-row">
        <span class="todo-icon">{{ allDone ? '✅' : '📋' }}</span>
        <span class="todo-title">{{ t('todoCard.title') }}</span>
        <span class="todo-progress">{{ doneCount }}/{{ totalCount }}</span>
      </div>
      <button class="collapse-btn" :title="t('todoCard.toggle')">
        <ChevronUp v-if="!collapsed" :size="16" />
        <ChevronDown v-else :size="16" />
      </button>
    </div>

    <!-- Collapsed summary -->
    <div v-if="collapsed && allDone" class="todo-collapsed-summary">
      {{ t('todoCard.allDoneSummary', { done: doneCount, total: totalCount }) }}
      <button class="expand-link" @click.stop="collapsed = false">
        {{ t('todoCard.expand') }} ▾
      </button>
    </div>

    <!-- Item list -->
    <div v-show="!collapsed" class="todo-items">
      <label
        v-for="item in items"
        :key="item.id"
        class="todo-item"
        :class="{ done: item.status === 'done' }"
      >
        <input
          type="checkbox"
          class="todo-checkbox"
          :checked="item.status === 'done'"
          @change="toggleItem(item)"
        />
        <span class="todo-check-icon" :class="{ checked: item.status === 'done' }">
          <Check v-if="item.status === 'done'" :size="12" />
        </span>
        <span class="todo-content">{{ item.content }}</span>
        <span v-if="item.status === 'done' && item.completed_at" class="todo-time">
          {{ formatTime(item.completed_at) }}
        </span>
      </label>
    </div>

    <!-- Mark all done button -->
    <div v-show="!collapsed && hasPending" class="todo-footer">
      <button class="mark-all-btn" @click="markAllDone">
        {{ t('todoCard.markAllDone') }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.todo-card {
  background: var(--bg-panel, var(--panel-solid, var(--panel)));
  border: 1px solid var(--line);
  border-radius: 12px;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  transition: all 0.2s ease;
  max-width: 400px;
}

.todo-card:hover {
  border-color: var(--accent-border);
  box-shadow: 0 4px 12px var(--accent-glow);
}

.todo-card.all-done {
  border-left: 4px solid var(--success);
}

/* Header */
.todo-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  cursor: pointer;
  user-select: none;
}

.todo-title-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.todo-icon {
  font-size: 1.1rem;
}

.todo-title {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--text);
}

.todo-progress {
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--accent);
  background: var(--accent-bg-light);
  padding: 0.125rem 0.5rem;
  border-radius: 9999px;
}

.collapse-btn {
  width: 28px;
  height: 28px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-sm, 6px);
  transition: all 0.15s ease;
}

.collapse-btn:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

/* Collapsed summary */
.todo-collapsed-summary {
  font-size: 0.85rem;
  color: var(--text-muted);
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.expand-link {
  background: none;
  border: none;
  color: var(--accent);
  cursor: pointer;
  font-size: 0.8rem;
  padding: 0;
  transition: opacity 0.15s ease;
}

.expand-link:hover {
  opacity: 0.8;
}

/* Items */
.todo-items {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.todo-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.625rem;
  border-radius: var(--radius-sm, 6px);
  cursor: pointer;
  transition: background 0.15s ease;
}

.todo-item:hover {
  background: var(--accent-bg-hover, var(--accent-bg-light));
}

.todo-item.done {
  opacity: 0.7;
}

.todo-checkbox {
  position: absolute;
  opacity: 0;
  width: 0;
  height: 0;
  pointer-events: none;
}

.todo-check-icon {
  width: 20px;
  height: 20px;
  min-width: 20px;
  border: 2px solid var(--line);
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
  color: transparent;
}

.todo-check-icon.checked {
  background: var(--success);
  border-color: var(--success);
  color: white;
}

.todo-content {
  flex: 1;
  font-size: 0.9rem;
  color: var(--text);
  transition: all 0.2s ease;
}

.todo-item.done .todo-content {
  text-decoration: line-through;
  color: var(--text-muted);
}

.todo-time {
  font-size: 0.75rem;
  color: var(--text-muted);
  white-space: nowrap;
}

/* Footer */
.todo-footer {
  display: flex;
  justify-content: flex-end;
  padding-top: 0.25rem;
  border-top: 1px solid var(--line);
}

.mark-all-btn {
  font-size: 0.8rem;
  font-weight: 500;
  color: var(--accent);
  background: var(--accent-bg-light);
  border: none;
  border-radius: var(--radius-sm, 6px);
  padding: 0.375rem 0.875rem;
  cursor: pointer;
  transition: all 0.15s ease;
}

.mark-all-btn:hover {
  transform: scale(1.02);
  filter: brightness(1.1);
}
</style>
