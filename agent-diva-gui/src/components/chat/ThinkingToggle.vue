<template>
  <div class="thinking-toggle" ref="containerRef">
    <button
      class="thinking-toggle-btn"
      :class="{ active: isOpen }"
      :disabled="disabled"
      :title="$t('chat.thinkingMode')"
      @click.stop="toggleDropdown"
    >
      <component :is="currentIcon" class="thinking-toggle-icon" />
    </button>
    <Transition name="dropdown">
      <div v-if="isOpen" class="thinking-toggle-dropdown" @click.stop>
        <button
          v-for="option in options"
          :key="option.value"
          class="thinking-toggle-option"
          :class="{ selected: modelValue === option.value }"
          @click="selectMode(option.value)"
        >
          <component :is="option.icon" class="thinking-toggle-option-icon" />
          <span class="thinking-toggle-option-label">{{ option.label }}</span>
          <span v-if="modelValue === option.value" class="thinking-toggle-check">✓</span>
        </button>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from 'vue'
import { useI18n } from 'vue-i18n'
import LightbulbAutoOutline from '../../assets/icons/LightbulbAutoOutline.vue'
import LightbulbOn from '../../assets/icons/LightbulbOn.vue'
import LightbulbOffOutline from '../../assets/icons/LightbulbOffOutline.vue'

type ThinkingMode = 'auto' | 'on' | 'off'

const props = defineProps<{
  modelValue: ThinkingMode
  disabled?: boolean
}>()

const emit = defineEmits<{
  'update:modelValue': [value: ThinkingMode]
}>()

const { t } = useI18n()

const isOpen = ref(false)
const containerRef = ref<HTMLElement | null>(null)

const currentIcon = computed(() => {
  switch (props.modelValue) {
    case 'on': return LightbulbOn
    case 'off': return LightbulbOffOutline
    default: return LightbulbAutoOutline
  }
})

const options = computed(() => [
  { value: 'auto' as ThinkingMode, icon: LightbulbAutoOutline, label: t('chat.thinkingModeAuto') },
  { value: 'on' as ThinkingMode, icon: LightbulbOn, label: t('chat.thinkingModeOn') },
  { value: 'off' as ThinkingMode, icon: LightbulbOffOutline, label: t('chat.thinkingModeOff') },
])

function toggleDropdown() {
  if (!props.disabled) {
    isOpen.value = !isOpen.value
  }
}

function selectMode(mode: ThinkingMode) {
  emit('update:modelValue', mode)
  isOpen.value = false
}

function handleClickOutside(e: MouseEvent) {
  if (containerRef.value && !containerRef.value.contains(e.target as Node)) {
    isOpen.value = false
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', handleClickOutside)
})
</script>

<style scoped>
.thinking-toggle {
  position: relative;
  display: inline-flex;
}

.thinking-toggle-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  cursor: pointer;
  padding: 4px;
  border-radius: var(--radius-sm);
  color: var(--text-muted);
  transition: background 0.15s, color 0.15s;
}

.thinking-toggle-btn:hover:not(:disabled) {
  background: var(--accent-bg-light);
  color: var(--text);
}

.thinking-toggle-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.thinking-toggle-btn.active {
  background: var(--accent-bg-hover);
  color: var(--accent);
}

.thinking-toggle-icon {
  font-size: 16px;
  width: 16px;
  height: 16px;
}

.thinking-toggle-dropdown {
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  margin-top: 4px;
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  min-width: 140px;
  z-index: 100;
  padding: 4px;
  overflow: hidden;
}

.thinking-toggle-option {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 10px;
  border: none;
  background: none;
  cursor: pointer;
  border-radius: var(--radius-sm);
  font-size: 13px;
  color: var(--text);
  transition: background 0.15s;
}

.thinking-toggle-option:hover {
  background: var(--accent-bg-light);
}

.thinking-toggle-option.selected {
  background: var(--accent-bg-hover);
  color: var(--accent);
}

.thinking-toggle-option-icon {
  font-size: 16px;
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.thinking-toggle-option-label {
  flex: 1;
}

.thinking-toggle-check {
  font-size: 12px;
  color: var(--accent);
}

.dropdown-enter-active,
.dropdown-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}

.dropdown-enter-from,
.dropdown-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(-4px);
}
</style>
