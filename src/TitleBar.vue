<script setup lang="ts">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMounted, onUnmounted, ref } from "vue";

const emit = defineEmits<{ maximized: [value: boolean] }>();
const desktop = isTauri();
const error = ref("");
const maximized = ref(false);
let unlisten: (() => void) | undefined;
let disposed = false;

onMounted(async () => {
  if (!desktop) return;
  try {
    const window = getCurrentWindow();
    const stop = await window.onResized(async () => {
      try { maximized.value = await window.isMaximized(); emit("maximized", maximized.value); } catch { /* Window may be closing. */ }
    });
    if (disposed) { stop(); return; }
    unlisten = stop;
    maximized.value = await window.isMaximized();
    emit("maximized", maximized.value);
  } catch {
    error.value = "Unable to read window state.";
  }
});
onUnmounted(() => { disposed = true; unlisten?.(); });

async function windowAction(action: "close" | "minimize" | "maximize") {
  if (!desktop) return;
  error.value = "";
  try {
    const window = getCurrentWindow();
    if (action === "close") await window.close();
    else if (action === "minimize") await window.minimize();
    else await window.toggleMaximize();
  } catch {
    error.value = "Window action failed. Please try again.";
  }
}
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <span class="title" data-tauri-drag-region>Agent Diva</span>
    <div class="window-controls" aria-label="Window controls">
      <button class="window-control" type="button" aria-label="Minimize" title="Minimize" :disabled="!desktop" @click="windowAction('minimize')">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h10" /></svg>
      </button>
      <button class="window-control" type="button" :aria-label="maximized ? 'Restore' : 'Maximize'" :title="maximized ? 'Restore' : 'Maximize'" :disabled="!desktop" @click="windowAction('maximize')">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path v-if="maximized" d="M5 4V2.5h8.5V11H12M2.5 5h9v8.5h-9z" /><path v-else d="M3 3h10v10H3z" /></svg>
      </button>
      <button class="window-control close" type="button" aria-label="Close" title="Close" :disabled="!desktop" @click="windowAction('close')">
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 3.5l9 9m0-9l-9 9" /></svg>
      </button>
    </div>
    <p v-if="error" class="window-error" role="alert">{{ error }}</p>
  </header>
</template>

<style scoped>
.titlebar { height: 48px; flex: 0 0 48px; display: flex; align-items: stretch; position: relative; border-bottom: 1px solid rgba(64, 77, 69, .12); background: rgba(246, 247, 241, .48); user-select: none; }
.title { flex: 1; display: flex; align-items: center; padding-left: 18px; font-size: 12px; font-weight: 600; color: #59675e; letter-spacing: .02em; }
.window-controls { display: flex; flex: 0 0 auto; }
.window-control { width: 46px; height: 100%; padding: 0; border: 0; background: transparent; color: #24332e; display: grid; place-items: center; cursor: pointer; }
.window-control svg { width: 12px; height: 12px; fill: none; stroke: currentColor; stroke-width: 1.2; }
.window-control:hover { background: rgba(42, 57, 48, .1); }
.window-control.close:hover { background: #c42b1c; color: white; }
.window-control:focus-visible { outline: 2px solid #526f60; outline-offset: -3px; }
.window-control:disabled { cursor: default; opacity: .5; }
.window-error { position: absolute; top: 32px; right: 18px; z-index: 2; padding: 8px 12px; background: #fff; color: #7c2929; border-radius: 6px; font-size: 12px; }
</style>
