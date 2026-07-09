<script setup lang="ts">
import type { MaskEntryDto } from '../api/desktop';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

const emit = defineEmits<{
  (e: 'select', mask: MaskEntryDto): void;
}>();

const props = defineProps<{
  mask: MaskEntryDto;
  active?: boolean;
  selectable?: boolean;
}>();

function handleClick() {
  emit('select', props.mask);
}

function modeLabel(mode: string): string {
  if (mode === 'assist') return t('mask.modeAssist');
  if (mode === 'normal') return t('mask.modeNormal');
  return mode || t('mask.modeNormal');
}

function modeColor(mode: string): string {
  if (mode === 'assist') return 'bg-purple-100 text-purple-700';
  return 'bg-gray-100 text-gray-600';
}
</script>

<template>
  <button
    type="button"
    class="relative flex flex-col items-center rounded-lg border p-3 text-left transition-all max-h-[160px] overflow-hidden w-full"
    :class="[
      active
        ? 'ring-2 ring-pink-500 border-pink-500 bg-pink-50'
        : 'border-gray-200 bg-white hover:shadow-md hover:border-gray-300',
      selectable !== false ? 'cursor-pointer' : 'cursor-default',
    ]"
    :disabled="selectable === false"
    @click="handleClick"
  >
    <!-- Active checkmark overlay -->
    <div
      v-if="active"
      class="absolute top-1.5 right-1.5 w-5 h-5 bg-pink-500 text-white rounded-full flex items-center justify-center z-10"
    >
      <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
      </svg>
    </div>

    <!-- Emoji icon -->
    <span class="text-4xl leading-none mb-2 mt-1">{{ mask.icon }}</span>

    <!-- Name -->
    <span class="text-sm font-semibold text-gray-800 truncate w-full text-center">{{ mask.name }}</span>

    <!-- Description -->
    <p class="text-xs text-gray-500 line-clamp-2 mt-1 text-center leading-tight">
      {{ mask.description }}
    </p>

    <!-- Badges row -->
    <div class="flex items-center justify-center gap-1.5 mt-auto pt-2 flex-wrap">
      <span
        class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium leading-tight"
        :class="modeColor(mask.mode)"
      >
        {{ modeLabel(mask.mode) }}
      </span>
      <span
        v-if="mask.readOnly"
        class="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-medium leading-tight bg-amber-100 text-amber-700"
      >
        {{ t('mask.readOnly') }}
      </span>
    </div>
  </button>
</template>
