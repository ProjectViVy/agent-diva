<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type { NeuroOverviewSnapshotV0 } from "../../api/neuro";
import { rowsForHemisphere } from "../../api/neuro";
import { deriveNeuroTroubleshootTemplate } from "../../api/neuroTroubleshooting";
import DataPhaseBadge from "./DataPhaseBadge.vue";
import NeuroTroubleshootCallout from "./NeuroTroubleshootCallout.vue";

const props = defineProps<{
  side: "left" | "right";
  snapshot: NeuroOverviewSnapshotV0 | null;
  loading?: boolean;
  loadError?: boolean;
  /** 浏览器等非桌面运行时传 false，避免展示无效的「关闭皮层」按钮。 */
  showDisableCortexAction?: boolean;
}>();

const emit = defineEmits<{
  (e: "retry"): void;
  (e: "back-to-chat"): void;
  (e: "open-settings"): void;
  (e: "disable-cortex"): void;
}>();

const { t } = useI18n();

const regionTitle = computed(() =>
  props.side === "left" ? t("neuro.regionCortexHub") : t("neuro.regionMotorBridge"),
);

const rows = computed(() =>
  props.snapshot ? rowsForHemisphere(props.snapshot, props.side) : [],
);

const troubleshootTemplate = computed(() =>
  deriveNeuroTroubleshootTemplate({
    loading: props.loading === true,
    loadError: props.loadError === true,
    snapshot: props.snapshot,
    side: props.side,
    showDisableCortexAction: props.showDisableCortexAction !== false,
  }),
);

const statusDotClass = (status: string) => {
  const s = status.toLowerCase();
  if (s === "active") return "bg-amber-500";
  if (s === "done") return "bg-emerald-500";
  if (s === "error") return "bg-red-500";
  return "bg-gray-300";
};
</script>

<template>
  <aside
    id="neuro-detail-panel-root"
    class="neuro-detail-panel flex min-h-0 w-full shrink-0 flex-col border-gray-200/90 bg-white/95 md:w-80 md:border-l"
    :aria-label="regionTitle"
    data-testid="neuro-detail-panel"
  >
    <div class="border-b border-gray-100 px-4 py-3">
      <h3 class="text-sm font-semibold text-gray-900">{{ regionTitle }}</h3>
      <p class="mt-0.5 text-[11px] text-gray-500">
        {{ t("neuro.detailPanelSubtitle") }}
      </p>
    </div>

    <div class="min-h-0 flex-1 space-y-3 overflow-auto px-4 py-3">
      <template v-if="loading">
        <p class="text-xs text-gray-500" data-testid="neuro-detail-loading">
          {{ t("neuro.detailLoading") }}
        </p>
      </template>

      <template v-else-if="!snapshot">
        <NeuroTroubleshootCallout
          v-if="troubleshootTemplate"
          :template="troubleshootTemplate"
          @retry="emit('retry')"
          @back-to-chat="emit('back-to-chat')"
          @open-settings="emit('open-settings')"
          @disable-cortex="emit('disable-cortex')"
        />
      </template>

      <template v-else>
        <div class="flex flex-wrap items-center gap-2">
          <DataPhaseBadge :phase="snapshot.dataPhase" />
        </div>

        <p
          v-if="snapshot.dataPhase === 'stub'"
          class="text-xs leading-relaxed text-amber-900/90"
        >
          {{ t("neuro.detailStubExplainer") }}
        </p>
        <p
          v-else-if="snapshot.dataPhase === 'degraded'"
          class="text-xs leading-relaxed text-slate-700"
        >
          {{ t("neuro.detailDegradedExplainer") }}
        </p>

        <ul
          v-if="rows.length > 0"
          class="space-y-2"
          data-testid="neuro-detail-rows"
        >
          <li
            v-for="r in rows"
            :key="r.id"
            class="flex gap-2 rounded-lg border border-gray-100 bg-white px-3 py-2 text-xs shadow-sm"
          >
            <span
              class="mt-1.5 h-2 w-2 shrink-0 rounded-full"
              :class="statusDotClass(r.status)"
              aria-hidden="true"
            />
            <div class="min-w-0 flex-1">
              <div class="font-medium text-gray-900">{{ r.label }}</div>
              <div v-if="r.detail" class="mt-0.5 text-[11px] text-gray-500">
                {{ r.detail }}
              </div>
            </div>
          </li>
        </ul>

        <NeuroTroubleshootCallout
          v-else-if="troubleshootTemplate"
          :template="troubleshootTemplate"
          @retry="emit('retry')"
          @back-to-chat="emit('back-to-chat')"
          @open-settings="emit('open-settings')"
          @disable-cortex="emit('disable-cortex')"
        />
      </template>
    </div>
  </aside>
</template>
