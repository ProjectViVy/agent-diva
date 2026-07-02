# 11 — 重要代码示例

## 1. 类型定义 (`types.ts`)

```typescript
// ── 新增类型 ──────────────────────────────────────────────
export type GaussSceneId = 'transparent' | 'space' | 'home' | 'sea'

export interface GaussSceneEntry {
  id: GaussSceneId | string
  name: string
  path: string
  isDefault: boolean
}

// ── PetConfig 扩展 ────────────────────────────────────────
export interface PetConfig {
  // ... 现有字段 ...
  selectedGaussSceneId: GaussSceneId
  gaussSceneList: GaussSceneEntry[]
}

export const DEFAULT_PET_CONFIG: PetConfig = {
  // ... 现有默认值 ...
  selectedGaussSceneId: 'transparent',
  gaussSceneList: [
    { id: 'transparent', name: '透明背景', path: '', isDefault: true },
    { id: 'home',        name: '室内场景', path: 'vrm/scene/home.spz',  isDefault: true },
    { id: 'sea',         name: '海边场景', path: 'vrm/scene/sea.spz',   isDefault: true },
    { id: 'space',       name: '太空场景', path: 'vrm/scene/space.spz', isDefault: true },
  ],
}
```

---

## 2. DivaVrmAvatar — 背景场景集成

```typescript
// ── 新增 props ────────────────────────────────────────────
const props = defineProps<{
  // ... 现有 ...
  backgroundScene?: string
  backgroundSceneUrl?: string
}>()

// ── 防竞态场景同步 ────────────────────────────────────────
let sceneLoadSeq = 0

async function syncBackgroundScene() {
  const r = runtime.value
  const sid = props.backgroundScene
  if (!r || !sid) return

  const seq = ++sceneLoadSeq
  try {
    await r.setBackgroundScene(sid as GaussSceneId, props.backgroundSceneUrl)
    if (seq !== sceneLoadSeq) return
    console.log(`[DivaVrmAvatar] 场景: ${sid}`)
  } catch (err) {
    if (seq !== sceneLoadSeq) return
    console.warn('[DivaVrmAvatar] 场景失败:', err)
    try { await r.setBackgroundScene('transparent') } catch {}
  }
}

watch(() => [props.backgroundScene, props.backgroundSceneUrl], () => {
  void syncBackgroundScene()
})

// 模型加载成功后应用
watch(loadState, async (state) => {
  if (state === 'loaded') {
    if (props.backgroundScene) await syncBackgroundScene()
    if (props.idleAnimationEnabled ?? false) {
      idleAnimationManager.value?.startIdleLoop()
    }
  }
})
```

---

## 3. DivaPetView — 场景快速切换

```vue
<script setup lang="ts">
import { Image } from 'lucide-vue-next'
import { ref } from 'vue'

const showScenePicker = ref(false)

const SCENE_ICONS: Record<string, string> = {
  transparent: '🖼️', home: '🏠', sea: '🌊', space: '🚀',
}
function getSceneIcon(id: string) { return SCENE_ICONS[id] ?? '🌐' }
function selectScene(id: string) {
  petConfig.value.selectedGaussSceneId = id
  showScenePicker.value = false
}
function onClickOutside() { showScenePicker.value = false }
</script>

<template>
  <div class="avatar-section relative flex-1 min-h-0">
    <DivaVrmAvatar
      v-show="!desktopPetActive"
      :model-path="vrmModelPath"
      :mood="currentMood"
      :is-speaking="isSpeaking"
      :active="!desktopPetActive"
      :lip-sync-enabled="petConfig.vrmExpressionEnabled"
      :background-scene="petConfig.selectedGaussSceneId"
    />

    <!-- 场景切换按钮 (齿轮旁边) -->
    <button
      class="absolute top-3 left-11 w-7 h-7 flex items-center justify-center
             rounded-full bg-white/70 backdrop-blur-sm shadow-sm border
             border-gray-100 text-gray-400 hover:text-pink-500
             hover:bg-white hover:border-pink-200 transition-all"
      title="切换场景"
      @click.stop="showScenePicker = !showScenePicker"
    >
      <Image :size="14" />
    </button>

    <!-- 场景下拉 -->
    <Transition name="menu-fade">
      <div v-if="showScenePicker"
           class="absolute top-11 left-3 z-20 min-w-[140px] py-1
                  bg-white/95 backdrop-blur-sm rounded-lg border
                  border-gray-100 shadow-lg"
           @click.stop>
        <div v-for="s in petConfig.gaussSceneList" :key="s.id"
             class="flex items-center gap-2 px-3 py-2 text-xs cursor-pointer
                    transition-colors hover:bg-pink-50"
             :class="s.id === petConfig.selectedGaussSceneId
                     ? 'text-pink-600 bg-pink-50' : 'text-gray-700'"
             @click="selectScene(s.id)">
          <span>{{ getSceneIcon(s.id) }}</span>
          <span>{{ s.name }}</span>
        </div>
      </div>
    </Transition>
    <!-- 点击外部关闭 (遮罩层) -->
    <div v-if="showScenePicker" class="fixed inset-0 z-10"
         @click="onClickOutside" />
    <!-- ... 其余现有模板 ... -->
  </div>
</template>
```

---

## 4. PetSettings — 场景配置

```vue
<script setup lang="ts">
const SCENE_OPTIONS = [
  { id: 'transparent', icon: '🖼️', label: '透明背景', desc: '默认模式，仅显示 VRM 模型和阴影' },
  { id: 'home',        icon: '🏠', label: '室内场景', desc: '温馨的室内环境，Gaussian Splat 渲染' },
  { id: 'sea',         icon: '🌊', label: '海边场景', desc: '阳光海滩环境，约 500K splats' },
  { id: 'space',       icon: '🚀', label: '太空场景', desc: '深邃太空环境，约 100K splats' },
]
</script>

<template>
  <!-- 在基本设置 section 之后 -->
  <section class="settings-card">
    <h3 class="settings-title">📺 3D 背景场景</h3>
    <div class="flex flex-col gap-2">
      <label v-for="opt in SCENE_OPTIONS" :key="opt.id"
             class="scene-option"
             :class="{ selected: petConfig.selectedGaussSceneId === opt.id }">
        <input type="radio" :value="opt.id"
               v-model="petConfig.selectedGaussSceneId"
               class="scene-radio" />
        <span class="scene-icon">{{ opt.icon }}</span>
        <div class="flex flex-col gap-0.5">
          <span class="scene-label">{{ opt.label }}</span>
          <span class="scene-desc">{{ opt.desc }}</span>
        </div>
      </label>
    </div>
    <p class="section-hint">💡 切换立即生效，场景加载约 1-3 秒</p>
  </section>
</template>
```

---

## 5. 运行时 API (已存在，无需修改)

```typescript
// avatar-runtime-vrm 暴露的公开 API
interface VrmRuntime {
  setBackgroundScene(sceneId: 'transparent' | 'space' | 'home' | 'sea' | string, url?: string): Promise<void>
  setShadowEnabled(enabled: boolean): void
  setRenderMode(mode: 'normal' | 'panorama'): void
}
```
