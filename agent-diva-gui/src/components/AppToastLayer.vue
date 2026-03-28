<script setup lang="ts">
import { computed } from 'vue';
import { CheckCircle2, AlertCircle, X } from 'lucide-vue-next';
import { dismissAppToast, getAppToast } from '../utils/appToast';

const toast = getAppToast();

const containerClass = computed(() => {
  if (toast.value?.tone === 'error') {
    return 'border-rose-200 bg-rose-50 text-rose-700 shadow-rose-200/60';
  }
  return 'border-emerald-200 bg-emerald-50 text-emerald-700 shadow-emerald-200/60';
});
</script>

<template>
  <Teleport to="body">
    <Transition
      enter-active-class="transition duration-200 ease-out"
      enter-from-class="translate-y-2 opacity-0"
      enter-to-class="translate-y-0 opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="translate-y-0 opacity-100"
      leave-to-class="translate-y-2 opacity-0"
    >
      <div
        v-if="toast"
        class="pointer-events-none fixed right-6 top-6 z-[650] max-w-sm"
      >
        <div
          class="pointer-events-auto flex items-start gap-3 rounded-2xl border px-4 py-3 shadow-xl backdrop-blur-sm"
          :class="containerClass"
        >
          <component :is="toast.tone === 'error' ? AlertCircle : CheckCircle2" :size="18" class="mt-0.5 shrink-0" />
          <div class="min-w-0 flex-1 text-sm font-medium leading-6">
            {{ toast.message }}
          </div>
          <button
            type="button"
            class="rounded-lg p-1 opacity-70 transition hover:bg-white/60 hover:opacity-100"
            @click="dismissAppToast"
          >
            <X :size="14" />
          </button>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>
