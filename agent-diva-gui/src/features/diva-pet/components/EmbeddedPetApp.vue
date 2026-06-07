<script setup lang="ts">
import { onMounted, onUnmounted, reactive } from 'vue'
import DivaVrmAvatar from '../vrm/components/DivaVrmAvatar.vue'

type EmbeddedPetMessage =
  | {
      type: 'diva-embedded-pet:state'
      payload: {
        modelPath: string
        backgroundScene?: string
        backgroundSceneUrl?: string
        mood?: string
        isSpeaking?: boolean
        active?: boolean
        lipSyncEnabled?: boolean
        transparentBackground?: boolean
        idleMotionEnabled?: boolean
        selectedMotionIds?: string[]
        startMotionId?: string
        startMotionToken?: string | number
        previewMotionId?: string | null
        stopPreviewToken?: number
      }
    }

const state = reactive({
  modelPath: '',
  backgroundScene: undefined as string | undefined,
  backgroundSceneUrl: undefined as string | undefined,
  mood: 'neutral',
  isSpeaking: false,
  active: true,
  lipSyncEnabled: false,
  transparentBackground: false,
  idleMotionEnabled: false,
  selectedMotionIds: [] as string[],
  startMotionId: 'appearing',
  startMotionToken: 0 as string | number,
  previewMotionId: null as string | null,
  stopPreviewToken: 0,
})

function applyState(payload: EmbeddedPetMessage['payload']): void {
  state.modelPath = payload.modelPath ?? ''
  state.backgroundScene = payload.backgroundScene
  state.backgroundSceneUrl = payload.backgroundSceneUrl
  state.mood = payload.mood ?? 'neutral'
  state.isSpeaking = !!payload.isSpeaking
  state.active = payload.active !== false
  state.lipSyncEnabled = !!payload.lipSyncEnabled
  state.transparentBackground = !!payload.transparentBackground
  state.idleMotionEnabled = !!payload.idleMotionEnabled
  state.selectedMotionIds = Array.isArray(payload.selectedMotionIds) ? payload.selectedMotionIds : []
  state.startMotionId = payload.startMotionId ?? 'appearing'
  state.startMotionToken = payload.startMotionToken ?? 0
  state.previewMotionId = payload.previewMotionId ?? null
  state.stopPreviewToken = payload.stopPreviewToken ?? 0
}

function onMessage(event: MessageEvent<EmbeddedPetMessage>): void {
  if (event.data?.type !== 'diva-embedded-pet:state') return
  applyState(event.data.payload)
}

onMounted(() => {
  window.addEventListener('message', onMessage)
  window.parent?.postMessage({ type: 'diva-embedded-pet:ready' }, '*')
})

onUnmounted(() => {
  window.removeEventListener('message', onMessage)
})
</script>

<template>
  <div class="embedded-pet-app">
    <DivaVrmAvatar
      v-if="state.modelPath"
      :key="state.transparentBackground ? 'transparent' : 'opaque'"
      :model-path="state.modelPath"
      :background-scene="state.backgroundScene"
      :background-scene-url="state.backgroundSceneUrl"
      :mood="state.mood"
      :is-speaking="state.isSpeaking"
      :active="state.active"
      :lip-sync-enabled="state.lipSyncEnabled"
      :transparent-background="state.transparentBackground"
      :idle-motion-enabled="state.idleMotionEnabled"
      :selected-motion-ids="state.selectedMotionIds"
      :start-motion-id="state.startMotionId"
      :start-motion-token="state.startMotionToken"
      :preview-motion-id="state.previewMotionId"
      :stop-preview-token="state.stopPreviewToken"
    />
  </div>
</template>

<style scoped>
.embedded-pet-app {
  width: 100%;
  height: 100%;
  background: transparent;
}
</style>
