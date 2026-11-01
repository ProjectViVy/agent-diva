<script setup lang="ts">
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ref } from "vue";

const desktop = isTauri();
const error = ref("");

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
    <div class="window-controls" aria-label="Window controls">
      <button class="window-control close" type="button" aria-label="Close" title="Close" :disabled="!desktop" @click="windowAction('close')"><span aria-hidden="true">×</span></button>
      <button class="window-control minimize" type="button" aria-label="Minimize" title="Minimize" :disabled="!desktop" @click="windowAction('minimize')"><span aria-hidden="true">−</span></button>
      <button class="window-control maximize" type="button" aria-label="Maximize or restore" title="Maximize or restore" :disabled="!desktop" @click="windowAction('maximize')"><span aria-hidden="true">↗</span></button>
    </div>
    <span class="title" data-tauri-drag-region>Agent Diva</span>
    <p v-if="error" class="window-error" role="alert">{{ error }}</p>
  </header>
</template>

<style scoped>
.titlebar { height: 48px; flex: 0 0 48px; display: flex; align-items: center; position: relative; border-bottom: 1px solid rgba(64, 77, 69, .12); background: rgba(246, 247, 241, .48); user-select: none; }
.window-controls { display: flex; gap: 8px; align-items: center; padding: 0 18px; z-index: 1; }
.window-control { width: 13px; height: 13px; padding: 0; border: 1px solid rgba(0, 0, 0, .12); border-radius: 50%; display: grid; place-items: center; cursor: pointer; color: rgba(41, 37, 30, .8); line-height: 1; }
.window-control span { opacity: 0; font: 11px/1 Arial, sans-serif; }
.window-controls:hover span, .window-control:focus-visible span { opacity: 1; }
.window-control:focus-visible { outline: 2px solid #526f60; outline-offset: 3px; }
.window-control:disabled { cursor: default; opacity: .5; }
.close { background: #ff6058; } .minimize { background: #ffbd2e; } .maximize { background: #28c840; }
.title { position: absolute; left: 110px; right: 110px; text-align: center; font-size: 12px; font-weight: 600; color: #59675e; letter-spacing: .02em; }
.window-error { position: absolute; top: 32px; left: 18px; z-index: 2; padding: 8px 12px; background: #fff; color: #7c2929; border-radius: 6px; font-size: 12px; }
</style>
