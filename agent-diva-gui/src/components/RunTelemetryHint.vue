<script setup lang="ts">
import { ChevronDown, ChevronUp } from 'lucide-vue-next';
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { RunTelemetrySnapshotV0 } from '../api/runTelemetry';

const { t } = useI18n();

const props = defineProps<{
  snapshot: RunTelemetrySnapshotV0 | null;
  loading?: boolean;
}>();

const expanded = ref(false);

const overBudget = computed(
  () => props.snapshot?.overSuggestedBudget === true,
);

const preludeCalls = (s: RunTelemetrySnapshotV0) => s.preludeLlmCalls ?? 0;

const lineSummary = computed(() => {
  const s = props.snapshot;
  if (!s) return t('advanced.runTelemetryEmpty');
  const base = t('advanced.runTelemetryLine', {
    mainLoop: s.internalStepCount,
    prelude: preludeCalls(s),
    phases: s.phaseCount,
  });
  if (s.fullSwarmConvergenceRounds != null) {
    return (
      base +
      t('advanced.runTelemetryLineConvergencePart', {
        n: s.fullSwarmConvergenceRounds,
      })
    );
  }
  return base;
});
</script>

<template>
  <div
    class="rounded-lg border text-sm transition-colors"
    :class="
      overBudget
        ? 'border-amber-400/80 bg-amber-50/90 text-amber-950'
        : 'border-slate-200 bg-slate-50/80 text-slate-800'
    "
    role="region"
    :aria-label="t('advanced.runTelemetryAria')"
  >
    <button
      type="button"
      class="w-full flex items-center justify-between gap-2 px-3 py-2 text-left font-medium"
      @click="expanded = !expanded"
    >
      <span class="flex items-center gap-2 min-w-0">
        <span
          v-if="overBudget"
          class="shrink-0 text-amber-700"
          aria-hidden="true"
        >⚠</span>
        <span class="truncate">{{ loading ? t('advanced.runTelemetryLoading') : lineSummary }}</span>
      </span>
      <component :is="expanded ? ChevronUp : ChevronDown" class="shrink-0 w-4 h-4 opacity-60" />
    </button>
    <div v-if="expanded && snapshot" class="px-3 pb-3 pt-0 space-y-1 text-xs opacity-90 border-t border-black/5">
      <p>{{ t('advanced.runTelemetrySchema', { v: snapshot.schemaVersion }) }}</p>
      <p>{{ t('advanced.runTelemetrySteps', { n: snapshot.internalStepCount }) }}</p>
      <p>{{ t('advanced.runTelemetryPrelude', { n: preludeCalls(snapshot) }) }}</p>
      <p>{{ t('advanced.runTelemetryPhases', { n: snapshot.phaseCount }) }}</p>
      <p v-if="snapshot.fullSwarmConvergenceRounds != null">
        {{ t('advanced.runTelemetryConvergence', { n: snapshot.fullSwarmConvergenceRounds }) }}
      </p>
      <p v-if="snapshot.overSuggestedBudget === true" class="text-amber-800 font-medium">
        {{ t('advanced.runTelemetryOverBudget') }}
      </p>
      <p v-else-if="snapshot.overSuggestedBudget === false" class="text-slate-600">
        {{ t('advanced.runTelemetryWithinBudget') }}
      </p>
    </div>
  </div>
</template>
