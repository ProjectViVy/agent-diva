# 02 - VRM 实现方案讲解

> Diva Pet VRM 模块的分阶段实现方案

---

## 1. 总览

> **参考项目**: [`super-agent-party/static/js/vrm.js`](../../../../../super-agent-party/static/js/vrm.js):1-4573（仅供架构参考，不可复制 AGPL-3.0 代码）  
> **所有实现基于 `@pixiv/three-vrm` (MIT) 官方文档和示例自主编写。**

```
Phase 1 ──────── Phase 2 ──────── Phase 3 ──────── Phase 4 ──────── Phase 5
基础设施        VRM 渲染          表情+动画         口型同步+集成    优化打磨
(0.5d)          (1.5d)            (1d)             (1.5d)           (0.5d)
```

---

## 2. Phase 1：基础设施搭建（0.5 天）

### 2.1 依赖安装

```bash
cd agent-diva-gui
pnpm add three @pixiv/three-vrm @pixiv/three-vrm-animation
pnpm add -D @types/three
```

### 2.2 模块目录

```bash
mkdir -p agent-diva-gui/src/features/diva-pet-vrm/{composables,services,components}
```

### 2.3 类型声明

```typescript
// src/features/diva-pet-vrm/shims-vrm.d.ts
declare module '@pixiv/three-vrm' {
  export { VRMLoaderPlugin, VRMUtils, VRMHumanoid, VRMExpressionManager }
  // ... 完整类型（库自带，此仅为兜底）
}
```

---

## 3. Phase 2：VRM 基础渲染（1.5 天）

### 3.1 步骤 2.1 — DivaVrmAvatar.vue（核心组件）

> **参考**: `@pixiv/three-vrm` 官方示例 + super-agent-party `vrm.js` 架构思路（不可复制代码）  
> **vrm.js 参考点**: L4313 `switchToModel()` 的 VRM 加载流程，L1780 加载后默认表情设置，L2120 `animate()` 渲染循环结构

基于 `@pixiv/three-vrm` 官方示例实现 Vue 3 组件：

```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import * as THREE from 'three'
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js'
import { VRMLoaderPlugin, VRMUtils } from '@pixiv/three-vrm'
import type { VRM } from '@pixiv/three-vrm'

// Props
const props = defineProps<{
  modelPath: string
  messages: Message[]
  isTyping: boolean
}>()

const emit = defineEmits<{
  (e: 'load-start'): void
  (e: 'load-success'): void
  (e: 'load-error', error: Error): void
}>()

// State
const containerRef = ref<HTMLDivElement | null>(null)
let renderer: THREE.WebGLRenderer
let scene: THREE.Scene
let camera: THREE.PerspectiveCamera
let vrm: VRM | null = null
let clock = new THREE.Clock()
let rafId = 0

// Load VRM
async function loadVrm(modelPath: string) {
  emit('load-start')

  const loader = new GLTFLoader()
  loader.register(parser => new VRMLoaderPlugin(parser, { autoUpdateHumanBones: true }))

  const gltf = await loader.loadAsync(modelPath)
  const loadedVrm = gltf.userData.vrm as VRM

  // 优化
  VRMUtils.removeUnnecessaryJoints(gltf.scene)
  VRMUtils.rotateVRM0(loadedVrm)  // VRM 0.x 模型自动旋转

  vrm = loadedVrm
  scene.add(vrm.scene)

  // 默认表情
  vrm.expressionManager.setValue('neutral', 1.0)

  emit('load-success')
}

// Setup Three.js
function setupRenderer(container: HTMLDivElement) {
  renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true })
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2))
  renderer.setSize(container.clientWidth, container.clientHeight)
  renderer.setClearColor(0x00000000, 0)
  container.appendChild(renderer.domElement)

  scene = new THREE.Scene()

  camera = new THREE.PerspectiveCamera(
    30, container.clientWidth / container.clientHeight, 0.1, 20
  )
  camera.position.set(0, 1.3, 2.5)
  camera.lookAt(0, 0.9, 0)

  // 光照
  const ambientLight = new THREE.AmbientLight(0xffffff, 0.6)
  scene.add(ambientLight)
  const directionalLight = new THREE.DirectionalLight(0xffffff, 0.8)
  directionalLight.position.set(1, 2, 3)
  scene.add(directionalLight)
}

// Render loop
function animate() {
  rafId = requestAnimationFrame(animate)
  const delta = Math.min(clock.getDelta(), 0.1)
  vrm?.update(delta)
  renderer.render(scene, camera)
}

// Lifecycle
onMounted(async () => {
  setupRenderer(containerRef.value!)
  await loadVrm(props.modelPath)
  animate()
})

onUnmounted(() => {
  cancelAnimationFrame(rafId)
  renderer?.dispose()
  vrm = null
})
</script>

<template>
  <div ref="containerRef" class="vrm-container" />
</template>
```

### 3.2 步骤 2.2 — VRM 模型加载服务

```typescript
// services/vrm-loader.ts
import { invoke } from '@tauri-apps/api/core'

export interface VrmModelInfo {
  id: string
  name: string
  path: string
  thumbnail?: string
}

#[tauri::command]  // Rust 端
async fn pet_list_vrm_models(app_handle: tauri::AppHandle) -> Result<Vec<VrmModelInfo>, String> {
    let vrm_dir = app_handle.path().resource_dir()?.join("vrm/models");
    // 扫描 .vrm 文件
}
```

---

## 4. Phase 3：表情与动画（1 天）

### 4.1 useVrmExpression composable

> **vrm.js 参考**: L1494-L1524（chunk animation 中的表情设置逻辑），L1656-L1677（render chunk 中的表情+BlendShape 混合，含 `EMOTIONS` 列表和 `limit` 限制）  
> **思路**: vrm.js 使用 `EMOTIONS` 列表 + `setValue(exp, val)` 控制 BlendShape 权重。我们基于此思路自主实现关键词匹配版本。

```typescript
// composables/useVrmExpression.ts
import { watch, computed } from 'vue'
import type { VRM } from '@pixiv/three-vrm'

const MOOD_KEYWORDS: Record<string, string[]> = {
  happy: ['开心', '哈哈', '太好了', '喜欢', '😊', 'great', 'happy'],
  sad: ['难过', '伤心', '遗憾', '😢', 'sorry', 'unfortunately'],
  angry: ['生气', '可恶', '愤怒', '😠'],
  surprised: ['哇', '天哪', '真的吗', '😲', 'wow', 'amazing'],
}

export function useVrmExpression(vrm: Ref<VRM | null>, messages: Ref<Message[]>) {
  const currentMood = ref<string>('neutral')

  const latestReply = computed(() =>
    messages.value.filter(m => m.role === 'agent').at(-1)
  )

  watch(latestReply, (reply) => {
    if (!reply?.content || !vrm.value) return

    // 简单关键词匹配推断情绪
    const text = reply.content.toLowerCase()
    let detected = 'neutral'

    for (const [mood, keywords] of Object.entries(MOOD_KEYWORDS)) {
      if (keywords.some(k => text.includes(k))) {
        detected = mood
        break
      }
    }

    currentMood.value = detected
    vrm.value.expressionManager.setValue(detected, 1.0)
  })

  return { currentMood }
}
```

### 4.2 useVrmAnimation composable

> **vrm.js 参考**: 
> - L1000 `loadVRMAAnimation()` — `.vrma` 加载 API 用法: `VRMAnimationLoaderPlugin` + `createVRMAnimationClip()`  
> - L536 `IdleAnimationManager` class — 空闲动画循环管理器的架构思路  
> - L1024 `loadIdleAnimations()` — 批量加载 .vrma 文件的管理方式

```typescript
// composables/useVrmAnimation.ts
import * as THREE from 'three'
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js'
import { VRMAnimationLoaderPlugin, createVRMAnimationClip } from '@pixiv/three-vrm-animation'
import type { VRM } from '@pixiv/three-vrm'

export function useVrmAnimation(vrm: Ref<VRM | null>) {
  const mixer = ref<THREE.AnimationMixer | null>(null)
  const currentAction = ref<THREE.AnimationAction | null>(null)

  watch(vrm, (newVrm) => {
    if (newVrm) {
      mixer.value = new THREE.AnimationMixer(newVrm.scene)
    }
  })

  async function playAnimation(animPath: string) {
    if (!vrm.value || !mixer.value) return

    const loader = new GLTFLoader()
    loader.register(parser => new VRMAnimationLoaderPlugin(parser))

    const gltf = await loader.loadAsync(animPath)
    const animData = gltf.userData.vrmAnimations[0]
    const clip = createVRMAnimationClip(animData, vrm.value)

    currentAction.value?.stop()
    const action = mixer.value.clipAction(clip)
    action.setLoop(THREE.LoopOnce, 1)
    action.clampWhenFinished = true
    action.play()
    currentAction.value = action
  }

  function stopAnimation() {
    currentAction.value?.stop()
  }

  // 在 render loop 中调用
  function update(delta: number) {
    mixer.value?.update(delta)
  }

  return { playAnimation, stopAnimation, update }
}
```

---

## 5. Phase 4：口型同步 + 集成（1.5 天）

### 5.1 useVrmMouthSync（简化版）

> **vrm.js 参考**: 
> - L1537 `startLipSyncForChunk()` — 完整音频分析口型同步（使用 Audio Analyser FFT，此为 P3 阶段方案）  
> - L1423 `animateChunk()` — 口型逐帧动画，交替切换 AIUEO 口型  
> - L1399 `getFormant()` — 共振峰检测（根据音频频率映射口型）  
> **MVP 策略**: 不实现音频分析，使用简版正弦波驱动口型开合，后续升级到 FFT 方案。

```typescript
// composables/useVrmMouthSync.ts
import type { VRM } from '@pixiv/three-vrm'

export function useVrmMouthSync(vrm: Ref<VRM | null>, isSpeaking: Ref<boolean>) {
  let mouthTimer = 0
  const MOUTH_SHAPES = ['aa', 'ih', 'ou'] as const

  function update(delta: number) {
    if (!vrm.value) return

    if (!isSpeaking.value) {
      MOUTH_SHAPES.forEach(s => vrm.value!.expressionManager.setValue(s, 0))
      return
    }

    mouthTimer += delta * 6  // 语速
    const value = 0.3 + 0.6 * Math.abs(Math.sin(mouthTimer))

    // 简单正弦波驱动口型
    const shapeIndex = Math.floor(mouthTimer) % MOUTH_SHAPES.length
    MOUTH_SHAPES.forEach((s, i) => {
      vrm.value!.expressionManager.setValue(s, i === shapeIndex ? value : 0)
    })
  }

  return { update }
}
```

### 5.2 集成到 DivaPetView

```vue
<template>
  <div class="diva-pet-view">
    <!-- 渲染器选择 -->
    <DivaVrmAvatar
      v-if="activeRenderer === 'vrm'"
      :model-path="vrmConfig.modelPath"
      :messages="messages"
      :is-typing="isTyping"
      :is-speaking="isSpeaking"
    />

    <DivaPetAvatar
      v-else-if="activeRenderer === 'live2d'"
      :model="live2dModel"
      ...
    />

    <!-- 共用：气泡、语音控制 -->
    <DivaPetBubble ... />
    <DivaPetVoicePanel ... />
  </div>
</template>
```

---

## 6. Phase 5：优化打磨（0.5 天）

- 自适应分辨率（DPR cap at 2）
- 页面不可见时暂停渲染
- 模型缓存（避免重复加载）
- 加载中/加载失败状态
- 窗口 resize 处理

---

## 7. 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `agent-diva-gui/package.json` | 修改 | 添加 `three` `@pixiv/three-vrm` |
| `agent-diva-gui/src/features/diva-pet-vrm/` | 新增 | 完整 VRM 模块 |
| `agent-diva-gui/public/vrm/` | 新增 | 默认 VRM 模型和动画 |
| `agent-diva-gui/src/components/DivaPetView.vue` | 修改 | 添加 VRM 渲染器切换 |
| `agent-diva-gui/src-tauri/src/commands.rs` | 修改 | 添加 `pet_list_vrm_models` |
| `agent-diva-gui/src/locales/zh.ts` | 修改 | VRM 相关翻译 |

**总工时：约 5 天**
