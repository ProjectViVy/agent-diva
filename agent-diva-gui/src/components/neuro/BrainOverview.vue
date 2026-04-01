<script setup lang="ts">
import { onMounted, onUnmounted, ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';

const { t } = useI18n();

/** Symmetric “brain” silhouette; left/right emphasis via clip paths in each card SVG. */
const BRAIN_PATH =
  'M20 60 C20 30 45 12 72 12 C88 12 98 22 100 38 C102 22 112 12 128 12 C155 12 180 30 180 60 C180 88 155 108 128 108 C112 108 102 98 100 82 C98 98 88 108 72 108 C45 108 20 88 20 60 Z';

const uid = useId().replace(/[^a-zA-Z0-9_-]/g, 'x');
const clipLeftStrong = `${uid}-ls`;
const clipLeftWeak = `${uid}-lw`;
const clipRightStrong = `${uid}-rs`;
const clipRightWeak = `${uid}-rw`;

const emit = defineEmits<{
  (e: 'select-hemisphere', side: 'left' | 'right'): void;
}>();

const HINT_STORAGE_KEY = 'agent-diva-neuro-brain-hint-seen';

const showFirstHint = ref(true);
const focusedSide = ref<'left' | 'right'>('left');

const dismissHint = () => {
  showFirstHint.value = false;
  try {
    localStorage.setItem(HINT_STORAGE_KEY, '1');
  } catch {
    /* ignore */
  }
};

const focusLeft = () => {
  focusedSide.value = 'left';
  emit('select-hemisphere', 'left');
};

const focusRight = () => {
  focusedSide.value = 'right';
  emit('select-hemisphere', 'right');
};

const onKeydown = (e: KeyboardEvent) => {
  const el = e.target as HTMLElement | null;
  if (el?.closest('input, textarea, select, [contenteditable="true"]')) {
    return;
  }
  if (e.key === 'ArrowLeft') {
    e.preventDefault();
    focusLeft();
  } else if (e.key === 'ArrowRight') {
    e.preventDefault();
    focusRight();
  }
};

onMounted(() => {
  try {
    if (localStorage.getItem(HINT_STORAGE_KEY) === '1') {
      showFirstHint.value = false;
    }
  } catch {
    /* ignore */
  }
  window.addEventListener('keydown', onKeydown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown);
});
</script>

<template>
  <div
    class="brain-overview flex h-full min-h-0 flex-col gap-4"
    role="region"
    :aria-label="t('neuro.brainOverviewAria')"
  >
    <p
      v-if="showFirstHint"
      class="shrink-0 rounded-lg border border-rose-100 bg-rose-50/80 px-3 py-2 text-xs text-rose-900/80"
    >
      {{ t('neuro.brainOverviewHint') }}
      <button
        type="button"
        class="ml-2 font-medium text-rose-700 underline decoration-rose-300 hover:text-rose-900"
        @click="dismissHint"
      >
        {{ t('neuro.dismissHint') }}
      </button>
    </p>

    <div class="grid min-h-0 flex-1 grid-cols-1 gap-4 md:grid-cols-2 md:gap-6">
      <button
        type="button"
        class="brain-region group flex min-h-[200px] flex-col rounded-xl border-2 border-transparent bg-white/90 p-4 text-left shadow-sm outline-none transition-all hover:border-rose-200 hover:shadow-md focus-visible:border-rose-400 focus-visible:ring-2 focus-visible:ring-rose-200"
        :class="focusedSide === 'left' ? 'border-rose-300 ring-2 ring-rose-100' : ''"
        @click="focusLeft"
      >
        <div class="mb-3 flex flex-1 items-center justify-center">
          <svg
            class="max-h-48 w-full text-gray-300 transition-colors group-hover:text-rose-200"
            viewBox="0 0 200 120"
            xmlns="http://www.w3.org/2000/svg"
            aria-hidden="true"
          >
            <defs>
              <clipPath :id="clipLeftStrong">
                <rect x="0" y="0" width="100" height="120" />
              </clipPath>
              <clipPath :id="clipLeftWeak">
                <rect x="100" y="0" width="100" height="120" />
              </clipPath>
            </defs>
            <path :d="BRAIN_PATH" fill="none" stroke="currentColor" stroke-width="1.2" />
            <g :clip-path="`url(#${clipLeftStrong})`">
              <path :d="BRAIN_PATH" fill="rgb(244 114 182)" fill-opacity="0.5" />
            </g>
            <g :clip-path="`url(#${clipLeftWeak})`">
              <path :d="BRAIN_PATH" fill="rgb(167 139 250)" fill-opacity="0.14" />
            </g>
            <line x1="100" y1="12" x2="100" y2="108" stroke="rgb(244 114 182)" stroke-width="1.5" stroke-dasharray="4 3" />
          </svg>
        </div>
        <h3 class="text-sm font-semibold text-gray-800">{{ t('neuro.regionCortexHub') }}</h3>
        <p class="mt-1 text-xs text-gray-500">{{ t('neuro.regionCortexHubDesc') }}</p>
      </button>

      <button
        type="button"
        class="brain-region group flex min-h-[200px] flex-col rounded-xl border-2 border-transparent bg-white/90 p-4 text-left shadow-sm outline-none transition-all hover:border-violet-200 hover:shadow-md focus-visible:border-violet-400 focus-visible:ring-2 focus-visible:ring-violet-200"
        :class="focusedSide === 'right' ? 'border-violet-300 ring-2 ring-violet-100' : ''"
        @click="focusRight"
      >
        <div class="mb-3 flex flex-1 items-center justify-center">
          <svg
            class="max-h-48 w-full text-gray-300 transition-colors group-hover:text-violet-200"
            viewBox="0 0 200 120"
            xmlns="http://www.w3.org/2000/svg"
            aria-hidden="true"
          >
            <defs>
              <clipPath :id="clipRightStrong">
                <rect x="100" y="0" width="100" height="120" />
              </clipPath>
              <clipPath :id="clipRightWeak">
                <rect x="0" y="0" width="100" height="120" />
              </clipPath>
            </defs>
            <path :d="BRAIN_PATH" fill="none" stroke="currentColor" stroke-width="1.2" />
            <g :clip-path="`url(#${clipRightStrong})`">
              <path :d="BRAIN_PATH" fill="rgb(167 139 250)" fill-opacity="0.5" />
            </g>
            <g :clip-path="`url(#${clipRightWeak})`">
              <path :d="BRAIN_PATH" fill="rgb(244 114 182)" fill-opacity="0.14" />
            </g>
            <line x1="100" y1="12" x2="100" y2="108" stroke="rgb(167 139 250)" stroke-width="1.5" stroke-dasharray="4 3" />
          </svg>
        </div>
        <h3 class="text-sm font-semibold text-gray-800">{{ t('neuro.regionMotorBridge') }}</h3>
        <p class="mt-1 text-xs text-gray-500">{{ t('neuro.regionMotorBridgeDesc') }}</p>
      </button>
    </div>
  </div>
</template>
