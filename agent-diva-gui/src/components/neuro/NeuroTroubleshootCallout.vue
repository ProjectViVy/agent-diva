<script setup lang="ts">
import { AlertCircle, Coffee, Inbox } from "lucide-vue-next";
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type {
  NeuroTroubleshootEmit,
  NeuroTroubleshootTemplate,
} from "../../api/neuroTroubleshooting";

const props = defineProps<{
  template: NeuroTroubleshootTemplate;
}>();

const emit = defineEmits<{
  (e: "retry"): void;
  (e: "back-to-chat"): void;
  (e: "open-settings"): void;
  (e: "disable-cortex"): void;
}>();

const { t } = useI18n();

const testId = computed(
  () => `neuro-troubleshoot-${props.template.variant}` as const,
);

const icon = computed(() => {
  switch (props.template.variant) {
    case "error":
      return AlertCircle;
    case "idle":
      return Coffee;
    default:
      return Inbox;
  }
});

const iconClass = computed(() => {
  switch (props.template.variant) {
    case "error":
      return "text-rose-600";
    case "idle":
      return "text-amber-700";
    default:
      return "text-slate-500";
  }
});

function dispatch(ev: NeuroTroubleshootEmit) {
  switch (ev) {
    case "retry":
      emit("retry");
      break;
    case "back-to-chat":
      emit("back-to-chat");
      break;
    case "open-settings":
      emit("open-settings");
      break;
    case "disable-cortex":
      emit("disable-cortex");
      break;
    default: {
      const _x: never = ev;
      void _x;
    }
  }
}
</script>

<template>
  <section
    class="rounded-xl border border-gray-200/90 bg-gray-50/90 px-3 py-3 shadow-sm"
    :data-testid="testId"
    role="region"
    :aria-labelledby="`${testId}-title`"
  >
    <div class="flex gap-3">
      <component
        :is="icon"
        :size="22"
        class="mt-0.5 shrink-0"
        :class="iconClass"
        aria-hidden="true"
      />
      <div class="min-w-0 flex-1 space-y-2">
        <h4
          :id="`${testId}-title`"
          class="text-sm font-semibold text-gray-900"
        >
          {{ t(template.titleKey) }}
        </h4>
        <p class="text-xs leading-relaxed text-gray-600">
          {{ t(template.bodyKey) }}
        </p>
        <div class="flex flex-wrap gap-2 pt-1">
          <button
            v-for="(a, idx) in template.suggestedActions"
            :key="`${a.labelKey}-${idx}`"
            type="button"
            class="rounded-lg border border-gray-200 bg-white px-2.5 py-1.5 text-xs font-medium text-gray-800 shadow-sm transition hover:border-pink-200 hover:bg-pink-50 hover:text-pink-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-pink-400 focus-visible:ring-offset-1"
            @click="dispatch(a.behavior.event)"
          >
            {{ t(a.labelKey) }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
