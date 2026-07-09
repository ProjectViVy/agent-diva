<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useMasks } from '../composables/useMasks';
import { Settings } from 'lucide-vue-next';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();
const { masks, activeMask, switchTo, refresh } = useMasks();

const isOpen = ref(false);

const emit = defineEmits<{
  (e: 'navigate-settings'): void;
}>();

onMounted(() => {
  refresh();
});

function togglePopover() {
  isOpen.value = !isOpen.value;
}

function closePopover() {
  isOpen.value = false;
}

async function handleSelect(name: string) {
  await switchTo(name);
  closePopover();
}

function handleManageMasks() {
  closePopover();
  emit('navigate-settings');
}

const activeIcon = () => activeMask.value?.icon || '😊';
const activeName = () => activeMask.value?.name || '';
</script>

<template>
  <div class="relative flex items-center">
    <!-- Trigger button -->
    <button
      class="flex items-center justify-center w-8 h-8 rounded-full border border-gray-200/50 bg-white hover:border-pink-200 hover:shadow-sm transition-all text-lg cursor-pointer flex-shrink-0"
      :title="activeName() || t('mask.selectMask')"
      @click.stop="togglePopover"
    >
      <span>{{ activeIcon() }}</span>
    </button>

    <!-- Backdrop -->
    <div
      v-if="isOpen"
      class="fixed inset-0 z-[190]"
      @click="closePopover"
    />

    <!-- Popover -->
    <Transition name="mask-popover">
      <div
        v-if="isOpen"
        class="absolute top-full right-0 mt-1 w-56 bg-white rounded-lg shadow-xl border border-gray-100 overflow-hidden z-[200]"
      >
        <div class="px-3 py-2 border-b border-gray-100">
          <span class="text-[10px] font-semibold text-gray-500 uppercase tracking-wider">{{ t('mask.title') }}</span>
        </div>

        <div class="py-1 max-h-64 overflow-y-auto">
          <button
            v-for="mask in masks"
            :key="mask.name"
            class="flex items-center gap-3 w-full px-3 py-2 text-left text-xs hover:bg-pink-50 transition-colors"
            :class="mask.name === activeMask?.name ? 'bg-pink-50/50 text-pink-600 font-medium' : 'text-gray-700'"
            @click="handleSelect(mask.name)"
          >
            <span class="text-lg flex-shrink-0">{{ mask.icon }}</span>
            <div class="flex-1 min-w-0">
              <div class="truncate">{{ mask.name }}</div>
              <div v-if="mask.description" class="text-[10px] text-gray-400 truncate">{{ mask.description }}</div>
            </div>
            <span v-if="mask.name === activeMask?.name" class="text-pink-500 text-xs flex-shrink-0">✓</span>
          </button>

          <div v-if="masks.length === 0" class="px-3 py-6 text-center text-gray-400 text-[10px]">
            {{ t('mask.noMasks') }}
          </div>
        </div>

        <div class="border-t border-gray-100">
          <button
            class="flex items-center gap-2 w-full px-3 py-2 text-left text-xs text-gray-500 hover:text-pink-600 hover:bg-pink-50 transition-colors"
            @click="handleManageMasks"
          >
            <Settings :size="12" />
            <span>{{ t('mask.manageMasks') }}</span>
          </button>
        </div>
      </div>
    </Transition>
  </div>
</template>

<style scoped>
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
