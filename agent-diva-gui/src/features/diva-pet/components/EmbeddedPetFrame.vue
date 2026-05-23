<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

interface Props {
  modelPath: string
  backgroundScene?: string
  backgroundSceneUrl?: string
  mood?: string
  isSpeaking?: boolean
  active?: boolean
  lipSyncEnabled?: boolean
  transparentBackground?: boolean
}

const props = defineProps<Props>()

const frame = ref<HTMLIFrameElement | null>(null)
const frameReady = ref(false)

const payload = computed(() => ({
  modelPath: props.modelPath,
  backgroundScene: props.backgroundScene,
  backgroundSceneUrl: props.backgroundSceneUrl,
  mood: props.mood ?? 'neutral',
  isSpeaking: !!props.isSpeaking,
  active: props.active !== false,
  lipSyncEnabled: !!props.lipSyncEnabled,
  transparentBackground: !!props.transparentBackground,
}))

function postState(): void {
  const target = frame.value?.contentWindow
  if (!target || !frameReady.value) return

  target.postMessage(
    {
      type: 'diva-embedded-pet:state',
      payload: payload.value,
    },
    '*',
  )
}

function onFrameLoad(): void {
  frameReady.value = true
  postState()
}

function onMessage(event: MessageEvent<{ type?: string }>): void {
  if (event.source !== frame.value?.contentWindow) return
  if (event.data?.type !== 'diva-embedded-pet:ready') return

  frameReady.value = true
  postState()
}

onMounted(() => {
  window.addEventListener('message', onMessage)
})

onUnmounted(() => {
  window.removeEventListener('message', onMessage)
})

watch(payload, () => {
  postState()
}, { deep: true, immediate: true })
</script>

<template>
  <iframe
    ref="frame"
    class="embedded-pet-frame"
    src="/embedded-pet.html"
    title="Diva Embedded Pet"
    @load="onFrameLoad"
  />
</template>

<style scoped>
.embedded-pet-frame {
  display: block;
  width: 100%;
  height: 100%;
  border: 0;
  background: transparent;
}
</style>
