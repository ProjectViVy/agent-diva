# 11 - VRM 代码示例

> 重要代码参考（基于 @pixiv/three-vrm MIT 官方文档，零 AGPL 代码）
>
> **参考项目**: [`super-agent-party/static/js/vrm.js`](../../../../../super-agent-party/static/js/vrm.js):1-4573（仅架构参考）
> - 关键函数索引见 `01-architecture-exploration.md` 第 1.2 节
> - 以下所有代码均为基于 `@pixiv/three-vrm` MIT 官方 API 自主编写

---

## 1. DivaVrmAvatar.vue — 核心 3D 渲染组件

```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import * as THREE from 'three'
import { OrbitControls } from 'three/addons/controls/OrbitControls.js'
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js'
import { VRMLoaderPlugin, VRMUtils } from '@pixiv/three-vrm'
import type { VRM } from '@pixiv/three-vrm'

interface Props {
  modelPath: string
  messages: Message[]
  isSpeaking?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  (e: 'load-start'): void
  (e: 'load-success', vrm: VRM): void
  (e: 'load-error', error: Error): void
}>()

const containerRef = ref<HTMLDivElement | null>(null)
const loadState = ref<'idle' | 'loading' | 'ready' | 'error'>('idle')
const loadProgress = ref(0)

let renderer: THREE.WebGLRenderer
let scene: THREE.Scene
let camera: THREE.PerspectiveCamera
let controls: OrbitControls
let vrm: VRM | null = null
let clock = new THREE.Clock()
let rafId = 0

// ── Three.js Setup ──────────────────────────────────────

function setupScene(container: HTMLDivElement) {
  // Renderer
  renderer = new THREE.WebGLRenderer({
    alpha: true,
    antialias: true,
    powerPreference: 'high-performance',
  })
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.setSize(container.clientWidth, container.clientHeight)
  renderer.setClearColor(0x00000000, 0)
  renderer.shadowMap.enabled = false  // MVP: 关闭阴影节省性能
  container.appendChild(renderer.domElement)

  // Scene
  scene = new THREE.Scene()

  // Camera
  camera = new THREE.PerspectiveCamera(
    30, container.clientWidth / container.clientHeight, 0.1, 20
  )
  camera.position.set(0, 1.3, 2.5)

  // Controls
  controls = new OrbitControls(camera, renderer.domElement)
  controls.target.set(0, 1.0, 0)
  controls.enableDamping = true
  controls.dampingFactor = 0.1
  controls.minDistance = 1.0
  controls.maxDistance = 5.0
  controls.update()

  // Lighting
  const ambient = new THREE.AmbientLight(0xffffff, 0.6)
  scene.add(ambient)

  const directional = new THREE.DirectionalLight(0xffffff, 0.8)
  directional.position.set(1, 2, 3)
  scene.add(directional)

  // Resize
  window.addEventListener('resize', onResize)
}

function onResize() {
  const container = containerRef.value
  if (!container) return
  camera.aspect = container.clientWidth / container.clientHeight
  camera.updateProjectionMatrix()
  renderer.setSize(container.clientWidth, container.clientHeight)
}

// ── VRM Loading ─────────────────────────────────────────

async function loadVrm(path: string): Promise<void> {
  loadState.value = 'loading'
  loadProgress.value = 0
  emit('load-start')

  try {
    const loader = new GLTFLoader()
    loader.register(parser =>
      new VRMLoaderPlugin(parser, { autoUpdateHumanBones: true })
    )

    // 进度回调
    const gltf = await loader.loadAsync(path, (event) => {
      if (event.total) {
        loadProgress.value = Math.round((event.loaded / event.total) * 100)
      }
    })

    const loadedVrm = gltf.userData.vrm as VRM
    if (!loadedVrm) throw new Error('文件不是有效的 VRM 格式')

    // 优化
    VRMUtils.removeUnnecessaryJoints(gltf.scene)
    if (loadedVrm.meta?.specVersion === '0.0') {
      VRMUtils.rotateVRM0(loadedVrm)  // VRM 0.x 自动旋转
    }

    // 清理旧模型
    if (vrm) {
      scene.remove(vrm.scene)
      vrm = null
      // Note: Three.js dispose 复杂，简化为移除引用
    }

    vrm = loadedVrm
    scene.add(vrm.scene)

    // 默认表情
    vrm.expressionManager?.setValue('neutral', 1.0)

    // 视线跟踪
    if (vrm.lookAt) {
      vrm.lookAt.target = camera
    }

    loadState.value = 'ready'
    emit('load-success', vrm)

    console.log(`[VRM] 模型加载完成: ${vrm.meta?.name ?? path}`)
  } catch (err) {
    console.error('[VRM] 加载失败:', err)
    loadState.value = 'error'
    emit('load-error', err instanceof Error ? err : new Error(String(err)))
  }
}

// ── Render Loop ─────────────────────────────────────────

function animate() {
  rafId = requestAnimationFrame(animate)
  const delta = Math.min(clock.getDelta(), 0.1)

  controls.update()
  vrm?.update(delta)
  renderer.render(scene, camera)
}

// ── Composition API Hooks ───────────────────────────────

watch(() => props.modelPath, (newPath) => {
  if (newPath) loadVrm(newPath)
})

onMounted(() => {
  setupScene(containerRef.value!)
  loadVrm(props.modelPath)
  animate()
})

onUnmounted(() => {
  cancelAnimationFrame(rafId)
  window.removeEventListener('resize', onResize)
  controls.dispose()
  renderer.dispose()
  vrm = null
})
</script>

<template>
  <div ref="containerRef" class="vrm-avatar">
    <!-- Loading -->
    <div v-if="loadState === 'loading'" class="vrm-avatar__overlay">
      <div class="vrm-avatar__spinner" />
      <span>加载中... {{ loadProgress }}%</span>
    </div>

    <!-- Error -->
    <div v-else-if="loadState === 'error'" class="vrm-avatar__overlay vrm-avatar__overlay--error">
      <span>角色加载失败</span>
      <button @click="loadVrm(props.modelPath)">重试</button>
    </div>
  </div>
</template>

<style scoped>
.vrm-avatar {
  width: 100%;
  height: 100%;
  position: relative;
}

.vrm-avatar__overlay {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: rgba(0,0,0,0.1);
  z-index: 10;
}

.vrm-avatar__spinner {
  width: 32px; height: 32px;
  border: 3px solid rgba(124, 58, 237, 0.2);
  border-top-color: #7c3aed;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin { to { transform: rotate(360deg); } }
</style>
```

---

## 2. useVrmExpression — 表情推断

```typescript
// composables/useVrmExpression.ts
import { ref, computed, watch } from 'vue'
import type { VRM } from '@pixiv/three-vrm'
import type { Message } from '../../../App.vue'

interface EmotionKeywords {
  [mood: string]: string[]
}

const DEFAULT_KEYWORDS: EmotionKeywords = {
  happy: ['哈哈', '开心', '太好了', '喜欢', '😊', '😄', 'great', 'happy', 'love'],
  sad: ['难过', '伤心', '遗憾', '😢', '😭', 'sorry', 'unfortunately', 'sad'],
  angry: ['生气', '可恶', '愤怒', '😠', '😡', 'angry'],
  surprised: ['哇', '天哪', '真的吗', '😲', '😮', 'wow', 'amazing', 'what'],
  relaxed: ['好的', '嗯嗯', '没关系', '不用谢', 'ok', 'fine'],
}

export function useVrmExpression(
  vrm: Ref<VRM | null>,
  messages: Ref<Message[]>,
  options?: { keywords?: EmotionKeywords }
) {
  const keywords = options?.keywords ?? DEFAULT_KEYWORDS
  const currentMood = ref<string>('neutral')
  const previousMood = ref<string>('neutral')

  const latestAgentReply = computed(() => {
    for (let i = messages.value.length - 1; i >= 0; i--) {
      if (messages.value[i].role === 'agent') return messages.value[i]
    }
    return null
  })

  function detectMood(text: string): string {
    const lower = text.toLowerCase()
    for (const [mood, words] of Object.entries(keywords)) {
      for (const word of words) {
        if (lower.includes(word)) return mood
      }
    }
    return 'neutral'
  }

  watch(latestAgentReply, (reply) => {
    if (!reply?.content || !vrm.value?.expressionManager) return

    const mood = detectMood(reply.content)

    if (mood !== previousMood.value) {
      // 淡入新表情
      vrm.value.expressionManager.setValue(mood, 1.0)
      // 重置旧表情（VRM 会自动混合，此处简化）
      if (previousMood.value !== 'neutral') {
        vrm.value.expressionManager.setValue(previousMood.value, 0)
      }
      previousMood.value = mood
      currentMood.value = mood
    }
  })

  // 手动设置表情
  function setExpression(expression: string, value: number = 1.0) {
    if (!vrm.value?.expressionManager) return
    vrm.value.expressionManager.setValue(expression, value)
    currentMood.value = expression
  }

  // 重置到默认
  function resetToNeutral() {
    if (!vrm.value?.expressionManager) return
    vrm.value.expressionManager.resetValues()
    vrm.value.expressionManager.setValue('neutral', 1.0)
    currentMood.value = 'neutral'
    previousMood.value = 'neutral'
  }

  return {
    currentMood,
    setExpression,
    resetToNeutral,
    detectMood,  // 暴露给外部使用
  }
}
```

---

## 3. useVrmMouthSync — 口型同步

```typescript
// composables/useVrmMouthSync.ts
import type { VRM } from '@pixiv/three-vrm'

export function useVrmMouthSync(
  vrm: Ref<VRM | null>,
  isSpeaking: Ref<boolean>
) {
  const MOUTH_SHAPES = ['aa', 'ih', 'ou', 'ee', 'oh'] as const
  let mouthTimer = 0
  let wasSpeaking = false

  function update(delta: number) {
    if (!vrm.value?.expressionManager) return

    // 停止说话 → 重置口型
    if (!isSpeaking.value) {
      if (wasSpeaking) {
        MOUTH_SHAPES.forEach(s => vrm.value!.expressionManager.setValue(s, 0))
        wasSpeaking = false
      }
      return
    }

    wasSpeaking = true
    mouthTimer += delta * 7  // 语速倍率

    // 简单正弦波驱动口型循环
    const cycle = mouthTimer % MOUTH_SHAPES.length
    const currentShape = MOUTH_SHAPES[Math.floor(cycle)]
    const value = 0.3 + 0.6 * Math.abs(Math.sin(mouthTimer * Math.PI))

    // 只设置当前口型
    MOUTH_SHAPES.forEach(s => {
      vrm.value!.expressionManager.setValue(s, s === currentShape ? value : 0)
    })
  }

  return { update }
}
```

---

## 4. vrm-loader — 模型管理服务

```typescript
// services/vrm-loader.ts
import { invoke } from '@tauri-apps/api/core'

export interface VrmModelInfo {
  id: string
  name: string
  path: string
  author?: string
  license?: string
}

/** 列出所有可用的 VRM 模型 */
export async function listVrmModels(): Promise<VrmModelInfo[]> {
  try {
    return await invoke<VrmModelInfo[]>('pet_list_vrm_models')
  } catch {
    // Fallback: 读取 public/vrm/models/ 目录
    return getDefaultModels()
  }
}

function getDefaultModels(): VrmModelInfo[] {
  return [
    {
      id: 'alice',
      name: 'Alice',
      path: '/vrm/models/alice.vrm',
      author: 'Default',
    },
  ]
}

/** 获取模型文件的完整 URL */
export function resolveVrmUrl(relativePath: string): string {
  if (relativePath.startsWith('http')) return relativePath
  // Tauri 环境：使用 resource_dir
  return relativePath  // 前端由 Tauri IPC 处理路径解析
}
```
