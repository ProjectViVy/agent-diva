<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type { NeuroDataPhase } from "../../api/neuro";

const props = defineProps<{
  phase: NeuroDataPhase;
}>();

const { t } = useI18n();

const label = computed(() => {
  switch (props.phase) {
    case "live":
      return t("neuro.dataPhase.live");
    case "stub":
      return t("neuro.dataPhase.stub");
    case "degraded":
      return t("neuro.dataPhase.degraded");
    default:
      return props.phase;
  }
});

const badgeClass = computed(() => {
  switch (props.phase) {
    case "live":
      return "border-emerald-200 bg-emerald-50 text-emerald-900";
    case "stub":
      return "border-amber-200 bg-amber-50 text-amber-950";
    case "degraded":
      return "border-slate-300 bg-slate-100 text-slate-800";
    default:
      return "border-gray-200 bg-gray-50 text-gray-800";
  }
});
</script>

<template>
  <span
    class="data-phase-badge inline-flex max-w-full items-center rounded-md border px-2 py-0.5 text-[11px] font-semibold uppercase tracking-wide"
    :class="badgeClass"
    data-testid="data-phase-badge"
  >
    {{ label }}
  </span>
</template>
