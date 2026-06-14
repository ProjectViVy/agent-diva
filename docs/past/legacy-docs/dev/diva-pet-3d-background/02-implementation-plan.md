# 02 — 实现方案讲解

> **核心**: `avatar-runtime-vrm` 的 `GaussSceneController` 已完成全部底层实现。开发工作集中在 Vue 前端层的连接与 UI。

---

## 1. 实现单元

| 单元 | 内容 | 修改文件 | 工作量 |
|------|------|---------|--------|
| A | 依赖 + 资源 | package.json, public/vrm/scene/ | 0.5h |
| B | 类型 + 配置 | types.ts, pet-config.ts | 0.5h |
| C | VRM 组件集成 | DivaVrmAvatar.vue | 1h |
| D | DivaPetView UI | DivaPetView.vue | 1h |
| E | PetSettings UI | PetSettings.vue | 1h |

---

## 2. 单元 A: 依赖与资源

### 2.1 安装 @sparkjsdev/spark

```bash
cd agent-diva/agent-diva-gui
pnpm add @sparkjsdev/spark
```

**备选**: 若 npm 版与 Three 184 不兼容，从 `super-agent-party/static/libs/@sparkjsdev/spark/dist/spark.module.js` 复制到 `src/vendor/spark.module.js`，vite.config.ts 加 alias。

### 2.2 复制场景文件

```bash
cp super-agent-party/vrm/scene/*.spz agent-diva/agent-diva-gui/public/vrm/scene/
```

---

## 3. 单元 B: 类型扩展

**文件**: `src/features/diva-pet/types.ts`

```typescript
// 新增类型
export type GaussSceneId = 'transparent' | 'space' | 'home' | 'sea'

export interface GaussSceneEntry {
  id: GaussSceneId | string
  name: string
  path: string       // 相对 public/ 的路径 (transparent 为空)
  isDefault: boolean
}

// PetConfig 扩展
export interface PetConfig {
  // ... 现有 ...
  selectedGaussSceneId: GaussSceneId
  gaussSceneList: GaussSceneEntry[]
}

// DEFAULT_PET_CONFIG 扩展
export const DEFAULT_PET_CONFIG: PetConfig = {
  // ... 现有 ...
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

## 4. 单元 C: DivaVrmAvatar 集成

**文件**: `src/features/diva-pet/vrm/components/DivaVrmAvatar.vue`

### 4.1 新增 props

```typescript
const props = defineProps<{
  // ... 现有 ...
  /** 3D 背景场景 ID ('transparent' 为无背景) */
  backgroundScene?: string
  /** 自定义场景 .spz 文件 URL */
  backgroundSceneUrl?: string
}>()
```

### 4.2 场景同步（防竞态）

```typescript
let sceneLoadSeq = 0

async function syncBackgroundScene() {
  const r = runtime.value
  const sid = props.backgroundScene
  if (!r || !sid) return

  const seq = ++sceneLoadSeq
  try {
    await r.setBackgroundScene(sid as GaussSceneId, props.backgroundSceneUrl)
    if (seq !== sceneLoadSeq) return
  } catch (err) {
    if (seq !== sceneLoadSeq) return
    console.warn('[DivaVrmAvatar] 场景设置失败:', err)
    try { await r.setBackgroundScene('transparent') } catch {}
  }
}

watch(() => [props.backgroundScene, props.backgroundSceneUrl], () => {
  void syncBackgroundScene()
})

// 模型加载成功后应用（在现有 watch loadState 中追加）
watch(loadState, async (state) => {
  if (state === 'loaded') {
    if (props.backgroundScene) await syncBackgroundScene()
    // ... 现有 idle loop 逻辑 ...
  }
})
```

### 4.3 DivaPetView 传递 prop

```vue
<!-- DivaPetView.vue 中 -->
<DivaVrmAvatar
  ...
  :background-scene="petConfig.selectedGaussSceneId"
/>
```

---

## 5. 单元 D: DivaPetView 场景快捷切换

**文件**: `src/features/diva-pet/components/DivaPetView.vue`

在模型管理器按钮（Settings 齿轮）下方添加场景快速切换按钮。点击时展开下拉：

```vue
<!-- 场景快速切换按钮 -->
<button
  class="absolute top-3 left-11 w-7 h-7 ..."
  title="切换场景"
  @click="showScenePicker = !showScenePicker"
>
  <Image :size="14" />
</button>

<!-- 场景下拉菜单 -->
<Transition name="menu-fade">
  <div v-if="showScenePicker" class="absolute top-11 left-3 z-20 ...">
    <div v-for="s in petConfig.gaussSceneList" :key="s.id"
         class="scene-pick-item"
         :class="{ active: s.id === petConfig.selectedGaussSceneId }"
         @click="selectScene(s.id)">
      {{ getSceneIcon(s.id) }} {{ s.name }}
    </div>
  </div>
</Transition>
```

`getSceneIcon()`: 映射 `transparent→🖼️, home→🏠, sea→🌊, space→🚀`

`selectScene(id)`: 设置 `petConfig.selectedGaussSceneId = id` → watch 自动同步到运行时。

---

## 6. 单元 E: PetSettings 场景配置

**文件**: `src/components/settings/PetSettings.vue`

在现有 "基本设置" section 后新增场景配置 section：

```vue
<section class="settings-card">
  <h3 class="settings-title">📺 3D 背景场景</h3>
  <div class="scene-options">
    <label v-for="opt in SCENE_OPTIONS" :key="opt.id"
           class="scene-option"
           :class="{ selected: petConfig.selectedGaussSceneId === opt.id }">
      <input type="radio" :value="opt.id"
             v-model="petConfig.selectedGaussSceneId" />
      <span class="scene-icon">{{ opt.icon }}</span>
      <div class="scene-info">
        <span class="scene-label">{{ opt.label }}</span>
        <span class="scene-desc">{{ opt.desc }}</span>
      </div>
    </label>
  </div>
  <p class="section-hint">💡 切换立即生效，场景加载约 1-3 秒</p>
</section>
```

---

## 7. 执行顺序

```
A (依赖+资源) ──┐
                ├──→ C (VRM 集成) ──→ D (PetView UI)
B (类型+配置) ──┘                     └──→ E (PetSettings UI)
```

A/B 并行 → C 依赖 B → D/E 依赖 C（D/E 可并行）

---

## 8. 不修改范围

- `avatar-runtime-vrm/` — 运行时层 0 修改
- Tauri 后端 — 预设场景走 public/ 静态资源
- `NormalMode.vue` — 透传机制不变
- `DesktopPetOverlay.vue` — 桌面覆盖层模式不在本次范围
- `tauri.conf.json` — 主窗口配置无变化
