<script setup lang="ts">
/**
 * DesktopPetApp.vue — Top-level Vue app for the desktop pet pop-out window.
 *
 * This is a lightweight standalone app that renders the VRM avatar overlay
 * in a transparent Tauri window. It communicates with the main window via
 * Tauri events for emotion sync and message streaming.
 */
import { onErrorCaptured, ref } from 'vue'
import DesktopPetOverlay from './DesktopPetOverlay.vue'

const error = ref<string | null>(null)

onErrorCaptured((err) => {
  error.value = String(err)
  console.error('[DesktopPetApp]', err)
  return false // prevent propagation
})
</script>

<template>
  <DesktopPetOverlay v-if="!error" />
  <div v-else class="flex items-center justify-center w-full h-full text-red-400 text-xs p-4">
    {{ error }}
  </div>
</template>
