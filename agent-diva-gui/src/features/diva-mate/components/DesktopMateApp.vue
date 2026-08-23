<script setup lang="ts">
/**
 * DesktopMateApp.vue — Top-level Vue app for the desktop mate pop-out window.
 *
 * This is a lightweight standalone app that renders the VRM avatar overlay
 * in a transparent Tauri window. It communicates with the main window via
 * Tauri events for emotion sync and message streaming.
 */
import { onErrorCaptured, ref } from 'vue'
import DesktopMateOverlay from './DesktopMateOverlay.vue'

const error = ref<string | null>(null)

onErrorCaptured((err) => {
  error.value = String(err)
  console.error('[DesktopMateApp]', err)
  return false // prevent propagation
})
</script>

<template>
  <DesktopMateOverlay v-if="!error" />
  <div v-else class="flex items-center justify-center w-full h-full text-red-400 text-xs p-4">
    {{ error }}
  </div>
</template>
