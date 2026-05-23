<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { createVrmRuntime } from 'avatar-runtime-vrm'
import type { AvatarRuntimeMode } from '@morediva/shared-avatar-protocol'
import { DESKTOP_PET_DEFAULT_TRANSFORM } from '../../../../../avatar-runtime-vrm/src/runtime/constants'

const p = defineProps<{
  modelPath: string
  backgroundScene?: string
  backgroundSceneUrl?: string
  mood?: string
  isSpeaking?: boolean
  lipSyncEnabled?: boolean
  desktopPet?: boolean
  transparentBackground?: boolean
  active?: boolean
}>()

const emit = defineEmits<{
  loadStart: []
  loadSuccess: []
  loadError: [error: Error]
}>()

const container = ref<HTMLDivElement | null>(null)

type R = Awaited<ReturnType<typeof createVrmRuntime>>
let runtime: R | null = null
let destroyed = false
let ro: ResizeObserver | null = null
let loadSeq = 0
let sceneSeq = 0
let zoom = 1.0

onMounted(async () => {
  const el = container.value
  if (!el) return

  const mode: AvatarRuntimeMode = p.desktopPet ? 'desktop-pet' : 'embedded'
  const transparent = !!p.desktopPet || !!p.transparentBackground
  const runtimeOptions = {
    mode,
    transparent,
    allowInteraction: true,
    backgroundColor: transparent ? null : '#ffffff',
    maxFps: p.desktopPet ? 24 : 60,
  }

  runtime = await createVrmRuntime(el, runtimeOptions)
  if (destroyed) {
    runtime.destroy()
    runtime = null
    return
  }
  runtime.resize({ width: el.clientWidth, height: el.clientHeight })

  ro = new ResizeObserver(() => {
    const e = container.value
    if (!e || !runtime) return
    runtime.resize({ width: e.clientWidth, height: e.clientHeight })
  })
  ro.observe(el)

  await loadModel()
  await syncBackgroundScene()
})

onUnmounted(() => {
  destroyed = true
  ro?.disconnect()
  ro = null
  if (runtime) {
    runtime.destroy()
    runtime = null
  }
})

async function loadModel(): Promise<void> {
  if (!runtime || destroyed) return
  if (!p.modelPath) {
    return
  }

  const seq = ++loadSeq
  emit('loadStart')

  try {
    await runtime.loadCharacter({
      id: 'diva-demo',
      kind: 'vrm',
      modelSource: p.modelPath,
      displayName: 'Diva',
      initialMood: (p.mood ?? 'neutral') as any,
    })
    if (seq !== loadSeq) return
    await syncDesktopPetCamera()
    emit('loadSuccess')
  } catch (err: any) {
    if (seq !== loadSeq) return
    const error = err instanceof Error ? err : new Error(String(err))
    emit('loadError', error)
  }
}

async function loadScene(sceneId: string, url?: string): Promise<void> {
  if (!runtime || destroyed) return

  const seq = ++sceneSeq

  try {
    await (runtime as any).setBackgroundScene(sceneId, url || undefined)
    if (seq !== sceneSeq) return
  } catch (err: any) {
    if (seq !== sceneSeq) return
    try {
      await (runtime as any).setBackgroundScene('transparent')
    } catch {}
  }
}

async function syncBackgroundScene(): Promise<void> {
  if (!p.backgroundScene) {
    return
  }
  await loadScene(p.backgroundScene, p.backgroundSceneUrl)
}

watch(() => p.modelPath, () => {
  void loadModel()
})
watch(() => [p.backgroundScene, p.backgroundSceneUrl] as const, () => {
  void syncBackgroundScene()
})
watch(() => p.mood, (m) => {
  if (runtime) (runtime as any).setMood(m ?? 'neutral')
})
watch(() => p.isSpeaking, (s) => {
  if (runtime) void (runtime as any).setSpeechState({ speaking: !!s })
})
watch(() => p.active, (a) => {
  if (!runtime) return
  if (a) {
    runtime.resume()
    void syncDesktopPetCamera()
  } else {
    runtime.pause()
  }
})

function setScale(v: number) {
  zoom = Math.max(0.75, Math.min(1.6, v))
  if (runtime) void syncDesktopPetCamera()
}

function getScale() {
  return zoom
}

defineExpose({ setScale, getScale })

async function syncDesktopPetCamera(): Promise<void> {
  if (!runtime) return

  if (p.desktopPet) {
    await (runtime as any).setTransform({
      ...DESKTOP_PET_DEFAULT_TRANSFORM,
      scale: zoom,
    })
    return
  }

  await (runtime as any).setTransform({ scale: zoom })
}
</script>

<template>
  <div ref="container" style="width:100%;height:100%;overflow:hidden" />
</template>
