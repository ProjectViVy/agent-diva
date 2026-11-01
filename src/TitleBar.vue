<script setup lang="ts">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { onMounted, onUnmounted, ref } from "vue";

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
      try { maximized.value = await window.isMaximized(); } catch { /* Window may be closing. */ }
    });
    if (disposed) { stop(); return; }
    unlisten = stop;
    maximized.value = await window.isMaximized();
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
    <button class="menu-button" type="button" aria-label="菜单，暂未开放" title="菜单，暂未开放" disabled><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16" /></svg></button>
    <span class="drag-space" data-tauri-drag-region></span>
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
.titlebar { height: 48px; flex: 0 0 48px; display: flex; align-items: stretch; position: relative; border-bottom: 1px solid rgba(114, 68, 58, .14); background: rgba(255, 239, 236, .3); user-select: none; }
.title { display: flex; align-items: center; padding-left: 18px; font-size: 12px; font-weight: 600; color: #805b65; letter-spacing: .02em; }
.menu-button { align-self: center; width: 36px; height: 36px; margin-left: 8px; padding: 9px; border: 0; border-radius: 6px; background: transparent; color: #805b65; opacity: .6; cursor: not-allowed; }
.menu-button svg { display: block; width: 100%; height: 100%; fill: none; stroke: currentColor; stroke-width: 1.6; stroke-linecap: round; }
.drag-space { flex: 1; }
.window-controls { display: flex; flex: 0 0 auto; }
.window-control { width: 46px; height: 100%; padding: 0; border: 0; background: transparent; color: #533941; display: grid; place-items: center; cursor: pointer; }
.window-control svg { width: 12px; height: 12px; fill: none; stroke: currentColor; stroke-width: 1.2; }
.window-control:hover { background: rgba(109, 62, 54, .12); }
.window-control.close:hover { background: #c42b1c; color: white; }
.window-control:focus-visible { outline: 2px solid #b76f87; outline-offset: -3px; }
.window-control:disabled { cursor: default; opacity: .5; }
.window-error { position: absolute; top: 32px; right: 18px; z-index: 2; padding: 8px 12px; background: #fff; color: #7c2929; border-radius: 6px; font-size: 12px; }
</style>
