<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useMask, type MaskEntry } from '../composables/useMask';
import { Settings } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const { currentMask, currentIcon, masks, loadMasks, switchMask } = useMask();

const isOpen = ref(false);

const emit = defineEmits<{
  (e: 'navigate-masks'): void;
}>();

onMounted(() => {
  loadMasks();
});

const togglePopover = () => {
  isOpen.value = !isOpen.value;
};

const closePopover = () => {
  isOpen.value = false;
};

const handleSelect = async (mask: MaskEntry) => {
  await switchMask(mask.name);
  closePopover();
};

const handleManageMasks = () => {
  closePopover();
  emit('navigate-masks');
};
</script>

<template>
  <div class="mask-switcher-wrapper">
    <!-- Trigger button: emoji in avatar area -->
    <button
      class="mask-switcher-trigger"
      :title="currentMask.name"
      @click.stop="togglePopover"
    >
      <span class="mask-switcher-emoji">{{ currentIcon }}</span>
    </button>

    <!-- Backdrop -->
    <div
      v-if="isOpen"
      class="mask-switcher-backdrop"
      @click="closePopover"
    />

    <!-- Popover -->
    <Transition name="mask-popover">
      <div v-if="isOpen" class="mask-switcher-popover">
        <div class="mask-switcher-header">
          <span class="mask-switcher-title">{{ t('mask.switcher.title', '切换面具') }}</span>
        </div>

        <div class="mask-switcher-list">
          <button
            v-for="mask in masks"
            :key="mask.name"
            class="mask-switcher-item"
            :class="{ active: mask.name === currentMask.name }"
            @click="handleSelect(mask)"
          >
            <span class="mask-item-icon">{{ mask.icon }}</span>
            <div class="mask-item-info">
              <span class="mask-item-name">{{ mask.name }}</span>
              <span v-if="mask.description" class="mask-item-desc">{{ mask.description }}</span>
            </div>
            <span v-if="mask.name === currentMask.name" class="mask-item-check">✓</span>
          </button>
        </div>

        <div class="mask-switcher-footer">
          <button class="mask-manage-link" @click="handleManageMasks">
            <Settings :size="12" />
            <span>{{ t('mask.switcher.manage', '管理面具') }}</span>
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.mask-switcher-wrapper {
  position: relative;
  display: flex;
  align-items: center;
}

/* ── Trigger button ── */
.mask-switcher-trigger {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--line);
  background: var(--panel-solid);
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;
}

.mask-switcher-trigger:hover {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
  transform: scale(1.08);
}

.mask-switcher-emoji {
  font-size: 16px;
  line-height: 1;
}

/* ── Backdrop ── */
.mask-switcher-backdrop {
  position: fixed;
  inset: 0;
  z-index: 190;
}

/* ── Popover ── */
.mask-switcher-popover {
  position: absolute;
  top: calc(100% + 8px);
  left: 0;
  z-index: 200;
  width: 240px;
  background: var(--panel-solid);
  border: 1px solid var(--line);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  backdrop-filter: var(--glass-blur);
  overflow: hidden;
}

.mask-switcher-header {
  padding: 10px 14px 6px;
  border-bottom: 1px solid var(--line);
}

.mask-switcher-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* ── Mask list ── */
.mask-switcher-list {
  padding: 4px;
  max-height: 280px;
  overflow-y: auto;
}

.mask-switcher-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  border: none;
  background: transparent;
  cursor: pointer;
  transition: all 0.15s ease;
  text-align: left;
}

.mask-switcher-item:hover {
  background: var(--accent-bg-light);
}

.mask-switcher-item.active {
  background: var(--accent-bg-hover);
}

.mask-item-icon {
  font-size: 20px;
  line-height: 1;
  flex-shrink: 0;
}

.mask-item-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.mask-item-name {
  font-size: 13px;
  font-weight: 500;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mask-switcher-item.active .mask-item-name {
  color: var(--accent);
}

.mask-item-desc {
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mask-item-check {
  font-size: 12px;
  color: var(--accent);
  font-weight: 700;
  flex-shrink: 0;
}

/* ── Footer ── */
.mask-switcher-footer {
  padding: 4px;
  border-top: 1px solid var(--line);
}

.mask-manage-link {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  border: none;
  background: transparent;
  color: var(--text-muted);
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.mask-manage-link:hover {
  background: var(--accent-bg-light);
  color: var(--accent);
}

/* ── Transition ── */
.mask-popover-enter-active {
  transition: all 0.15s ease-out;
}

.mask-popover-leave-active {
  transition: all 0.1s ease-in;
}

.mask-popover-enter-from,
.mask-popover-leave-to {
  opacity: 0;
  transform: translateY(-4px) scale(0.97);
}
</style>
