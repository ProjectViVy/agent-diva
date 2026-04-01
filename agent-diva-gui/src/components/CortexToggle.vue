<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Brain, Loader2 } from "lucide-vue-next";
import { listen, type Event, type UnlistenFn } from "@tauri-apps/api/event";
import {
  CORTEX_SYNC_REJECTED,
  getCortexState,
  toggleCortex,
  type CortexState,
} from "../api/cortex";
import { showAppToast } from "../utils/appToast";

const { t } = useI18n();

const enabled = ref(false);
const pending = ref(false);
/** 已从后端拿到可信快照，开关语义与真相源一致 */
const ready = ref(false);
const loadingInitial = ref(true);
const loadError = ref(false);
let unlisten: UnlistenFn | null = null;

function applySnapshot(s: CortexState) {
  if (s.schemaVersion !== 0) {
    console.warn("[CortexToggle] unexpected schemaVersion", s.schemaVersion);
  }
  enabled.value = s.enabled;
}

function messageFromInvoke(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "";
}

function userFacingToggleError(err: unknown): string {
  const raw = messageFromInvoke(err);
  if (raw === CORTEX_SYNC_REJECTED) {
    return t("cortex.toggleSyncFailedCodeRejected");
  }
  return t("cortex.toggleSyncFailed");
}

async function refreshFromBackend() {
  loadingInitial.value = true;
  loadError.value = false;
  try {
    const snap = await getCortexState();
    applySnapshot(snap);
    ready.value = true;
  } catch (e) {
    console.error("[CortexToggle] get_cortex_state failed", e);
    ready.value = false;
    loadError.value = true;
  } finally {
    loadingInitial.value = false;
  }
}

/**
 * 以后端确认为准（架构 State Management）：不在 invoke 成功前翻转 UI，避免「假成功」与未知中间态（NFR-R1）。
 */
async function onToggle() {
  if (pending.value || !ready.value) return;
  pending.value = true;
  try {
    const snap = await toggleCortex();
    applySnapshot(snap);
  } catch (e) {
    console.error("[CortexToggle] toggle_cortex failed", e);
    showAppToast(userFacingToggleError(e), "error", 5200);
    try {
      const snap = await getCortexState();
      applySnapshot(snap);
    } catch (e2) {
      console.error("[CortexToggle] resync after failure failed", e2);
    }
  } finally {
    pending.value = false;
  }
}

onMounted(async () => {
  await refreshFromBackend();
  unlisten = await listen<CortexState>("cortex_toggled", (ev: Event<CortexState>) => {
    applySnapshot(ev.payload);
    ready.value = true;
    loadError.value = false;
  });
});

onUnmounted(() => {
  if (unlisten) {
    unlisten();
    unlisten = null;
  }
});
</script>

<template>
  <div
    class="cortex-toggle flex items-center gap-3 select-none"
    data-testid="cortex-toggle"
  >
    <div
      id="cortex-toggle-label"
      class="flex items-center gap-2 min-w-0"
    >
      <Brain
        :size="22"
        class="flex-shrink-0 transition-colors duration-150"
        :class="
          enabled
            ? 'text-pink-600'
            : 'text-gray-400'
        "
        aria-hidden="true"
      />
      <div class="flex flex-col min-w-0 leading-tight">
        <span class="text-xs font-semibold text-gray-700 truncate">
          {{ t("cortex.layerLabel") }}
        </span>
        <span
          class="text-[11px] font-medium"
          :class="
            loadingInitial
              ? 'text-gray-400'
              : loadError
                ? 'text-amber-700'
                : enabled
                  ? 'text-pink-600'
                  : 'text-gray-500'
          "
        >
          {{
            loadingInitial
              ? t("cortex.loadingState")
              : loadError
                ? t("cortex.loadFailedShort")
                : enabled
                  ? t("cortex.stateOn")
                  : t("cortex.stateOff")
          }}
        </span>
      </div>
    </div>
    <div
      v-if="loadingInitial"
      class="flex h-7 w-12 flex-shrink-0 items-center justify-center"
      aria-live="polite"
      aria-busy="true"
    >
      <Loader2 :size="20" class="animate-spin text-gray-400" aria-hidden="true" />
      <span class="sr-only">{{ t("cortex.loadingState") }}</span>
    </div>
    <button
      v-else-if="loadError"
      type="button"
      class="flex-shrink-0 rounded-md border border-amber-200 bg-amber-50 px-2.5 py-1 text-[11px] font-semibold text-amber-900 transition-colors hover:bg-amber-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-amber-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white"
      @click="refreshFromBackend"
    >
      {{ t("cortex.retryLoad") }}
    </button>
    <button
      v-else
      type="button"
      role="switch"
      :aria-checked="enabled"
      :aria-busy="pending"
      :aria-disabled="pending || !ready"
      aria-labelledby="cortex-toggle-label"
      :disabled="pending || !ready"
      class="relative inline-flex h-7 w-12 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-pink-500 focus-visible:ring-offset-2 focus-visible:ring-offset-white disabled:cursor-not-allowed disabled:opacity-60"
      :class="enabled ? 'bg-pink-500' : 'bg-gray-300'"
      @click="onToggle"
    >
      <span
        class="pointer-events-none inline-block h-6 w-6 transform rounded-full bg-white shadow ring-0 transition duration-200"
        :class="enabled ? 'translate-x-5' : 'translate-x-0.5'"
        style="margin-top: 1px"
      />
    </button>
  </div>
</template>
