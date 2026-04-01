<script setup lang="ts">
/**
 * Story 2.3：皮层开时消费 `swarm_process` SSE → Tauri `swarm-process-batch`（由父级注入 events）。
 * NFR-P2：展示层用 rAF 合并同一帧内多次 prop 更新，避免高频重绘挡主流式。
 */
import { ChevronDown, ChevronUp, ListTree } from 'lucide-vue-next';
import { computed, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';

import type { ProcessEventWire } from '../../types/swarmProcess';

const props = defineProps<{
  /** 当前流式请求的过程事件（已按 request 过滤） */
  events: ProcessEventWire[];
  /** 大脑皮层开且 Tauri 时为 true */
  show: boolean;
}>();

const { t } = useI18n();

const expanded = ref(true);
const displayEvents = ref<ProcessEventWire[]>([]);
let raf = 0;

function scheduleFlush() {
  if (raf) cancelAnimationFrame(raf);
  raf = requestAnimationFrame(() => {
    raf = 0;
    displayEvents.value = props.events.slice();
  });
}

watch(
  () => props.events,
  () => {
    scheduleFlush();
  },
  { deep: true, immediate: true }
);

onUnmounted(() => {
  if (raf) cancelAnimationFrame(raf);
});

const hasToolMilestone = computed(() =>
  displayEvents.value.some(
    (e) => e.name === 'tool_call_started' || e.name === 'tool_call_finished'
  )
);

const isCapped = computed(() =>
  displayEvents.value.some((e) => e.name === 'swarm_run_capped')
);

const isDone = computed(() =>
  displayEvents.value.some((e) => e.name === 'swarm_run_finished')
);

/** UX-DR4：无工具里程碑时视为轻量路径展示（单行摘要，不展开全链） */
const lightweightVisual = computed(
  () => !hasToolMilestone.value && !isCapped.value && displayEvents.value.length > 0
);

const statusLabel = computed(() => {
  if (isCapped.value) return t('processFeedback.statusCapped');
  if (isDone.value) return t('processFeedback.statusDone');
  return t('processFeedback.statusRunning');
});

const summaryLine = computed(() => {
  const evs = displayEvents.value;
  if (evs.length === 0) return '';
  const last = evs[evs.length - 1];
  if (last.name === 'tool_call_started' && last.toolName) {
    return t('processFeedback.summaryTool', { tool: last.toolName });
  }
  if (last.message) return last.message;
  return last.name;
});

const timelineSteps = computed(() => {
  const lines: string[] = [];
  for (const e of displayEvents.value) {
    if (e.name === 'swarm_phase_changed') {
      lines.push(e.message || e.phaseId || e.name);
    } else if (e.name === 'tool_call_started') {
      lines.push(t('processFeedback.stepToolStart', { name: e.toolName || 'tool' }));
    } else if (e.name === 'tool_call_finished') {
      lines.push(t('processFeedback.stepToolEnd', { name: e.toolName || 'tool' }));
    } else if (e.name === 'swarm_run_finished') {
      lines.push(t('processFeedback.stepFinished', { reason: e.stopReason || '—' }));
    } else if (e.name === 'swarm_run_capped') {
      lines.push(t('processFeedback.stepCapped'));
    }
  }
  return lines;
});

const liveRegionText = computed(() => {
  const evs = displayEvents.value;
  if (evs.length === 0) return '';
  const last = evs[evs.length - 1];
  if (last.name === 'swarm_phase_changed' || last.name === 'swarm_run_capped') {
    return `${statusLabel.value}. ${last.message || ''}`;
  }
  return '';
});
</script>

<template>
  <div
    v-if="show && displayEvents.length > 0"
    class="process-feedback-strip neuro-surface border-b border-[var(--process-muted-border)] bg-[var(--process-muted-bg)] px-3 py-1.5 z-[8] flex flex-col gap-1 text-[var(--process-muted-fg)]"
    data-testid="process-feedback-strip"
  >
    <div class="flex items-center justify-between gap-2 min-h-[28px]">
      <div class="flex items-center gap-2 min-w-0">
        <ListTree class="w-3.5 h-3.5 shrink-0 opacity-70" aria-hidden="true" />
        <span class="text-[11px] font-medium uppercase tracking-wide opacity-80 truncate">
          {{ statusLabel }}
        </span>
      </div>
      <button
        type="button"
        class="shrink-0 rounded border border-gray-400/40 px-2 py-0.5 text-[10px] text-gray-600 hover:bg-white/60 focus:outline-none focus:ring-1 focus:ring-pink-400"
        data-testid="process-feedback-toggle"
        @click="expanded = !expanded"
      >
        <span class="inline-flex items-center gap-1">
          {{ expanded ? t('processFeedback.collapse') : t('processFeedback.expand') }}
          <ChevronUp v-if="expanded" class="w-3 h-3" />
          <ChevronDown v-else class="w-3 h-3" />
        </span>
      </button>
    </div>

    <div
      v-if="expanded"
      class="text-[11px] leading-snug pl-5 space-y-0.5 max-h-[88px] overflow-y-auto scrollbar-thin"
    >
      <p v-if="lightweightVisual" class="text-gray-600 truncate" :title="summaryLine">
        {{ summaryLine }}
      </p>
      <ul v-else class="list-disc list-inside space-y-0.5 text-gray-600">
        <li v-for="(step, i) in timelineSteps" :key="i" class="truncate" :title="step">
          {{ step }}
        </li>
      </ul>
    </div>

    <span class="sr-only" aria-live="polite">{{ liveRegionText }}</span>
  </div>
</template>

<style scoped>
.neuro-surface {
  --process-muted-bg: rgb(248 250 252 / 0.92);
  --process-muted-fg: rgb(71 85 105);
  --process-muted-border: rgb(203 213 225 / 0.6);
}
</style>
