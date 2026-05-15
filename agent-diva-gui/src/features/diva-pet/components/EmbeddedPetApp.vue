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
})

function applyState(payload: EmbeddedPetMessage['payload']): void {
  state.modelPath = payload.modelPath ?? ''
  state.backgroundScene = payload.backgroundScene
  state.backgroundSceneUrl = payload.backgroundSceneUrl
  state.mood = payload.mood ?? 'neutral'
  state.isSpeaking = !!payload.isSpeaking
  state.active = payload.active !== false
  state.lipSyncEnabled = !!payload.lipSyncEnabled
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
      :model-path="state.modelPath"
      :background-scene="state.backgroundScene"
      :background-scene-url="state.backgroundSceneUrl"
      :mood="state.mood"
      :is-speaking="state.isSpeaking"
      :active="state.active"
      :lip-sync-enabled="state.lipSyncEnabled"
    />
  </div>
</template>

<style scoped>
.embedded-pet-app {
  width: 100%;
  height: 100%;
  background: #ffffff;
}
</style>
