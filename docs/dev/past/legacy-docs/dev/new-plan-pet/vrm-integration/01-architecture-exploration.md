# 01 - VRM 架构设计探索

> 调研 super-agent-party 的 VRM (3D Avatar) 能力整合到 Agent Diva 的技术架构方案

---

## 1. 现有系统架构分析

### 1.1 Agent Diva GUI 架构（回顾）

```
App.vue — 单一数据源
├── messages: Ref<Message[]>          ← 所有消息
├── currentChatId / currentSessionKey
├── sendMessage(content)              ← 唯一发送入口
│
└── NormalMode.vue
    ├── ChatView.vue    (v-if activeTab === 'chat')
    ├── SettingsView    (v-if activeTab === 'settings')
    └── [新] DivaPetView (v-if activeTab === 'pet')
         ├── Live2D Avatar (2D Canvas)
         └── [新] VRM Avatar (3D Three.js)
```

### 1.2 super-agent-party VRM 架构

> **参考源文件**: [`super-agent-party/static/js/vrm.js`](../../../../../super-agent-party/static/js/vrm.js):1-4573（188KB，AGPL-3.0 ⚠️ 不可直接复制）  
> **入口 HTML**: [`super-agent-party/static/vrm.html`](../../../../../super-agent-party/static/vrm.html):1-76  
> **模型资源**: [`super-agent-party/vrm/`](../../../../../super-agent-party/vrm/)（Alice.vrm, Bob.vrm, animations/*.vrma, scene/*.spz）

```
┌──────────────────────────────────────────────────────┐
│              super-agent-party VRM                   │
│                                                      │
│  static/vrm.html (入口, 76行)                         │
│  ├── <script type="importmap"> (L61-L72)             │
│  │   ├── three            → libs/three/              │
│  │   ├── @pixiv/three-vrm → libs/@pixiv/three-vrm/   │
│  │   └── @sparkjsdev/spark → libs/@sparkjsdev/spark/ │
│  │                                                   │
│  └── js/vrm.js (4573行, 188KB)  ← AGPL-3.0 ⚠️       │
│      ├── VRM 模型加载 (GLTFLoader + VRMLoaderPlugin) │
│      ├── 表情系统 (expressionManager → BlendShape)   │
│      ├── 口型同步 (Audio Analyser → mouth shapes)    │
│      ├── 动画系统 (VRMAnimationLoaderPlugin)         │
│      ├── 3D 场景 (Gaussian Splatting .spz)          │
│      ├── WebXR 全景支持                              │
│      ├── 相机控制 (Orbit/PointerLock/Transform)      │
│      └── Vue 3 + Element Plus 控制面板               │
│                                                      │
│  vrm/  (资源目录)                                    │
│  ├── Alice.vrm, Bob.vrm          ← VRM 模型文件      │
│  ├── animations/*.vrma           ← VRM 动画文件      │
│  └── scene/*.spz                 ← 3D 场景文件       │
└──────────────────────────────────────────────────────┘
```

**vrm.js 关键函数索引**（仅供架构参考，不可复制源码）：

| 功能域 | 函数/类 | 行号 | 说明 |
|--------|---------|------|------|
| 模型管理 | `getAllModels()` | ~L4292 | 获取可用模型列表 |
| 模型管理 | `switchToModel()` | ~L4313 | 切换到指定模型 |
| 动画加载 | `loadVRMAAnimation()` | ~L1000 | 加载单个 .vrma 文件 |
| 动画管理 | `IdleAnimationManager` class | ~L536 | 空闲动画循环管理器 |
| 动画生成 | `createIdleClip()` | ~L1096 | 程序化空闲动画 |
| 动画生成 | `createBreathClip()` | ~L1254 | 程序化呼吸动画 |
| 动画生成 | `createBlinkClip()` | ~L1282 | 程序化眨眼动画 |
| 口型同步 | `startLipSyncForChunk()` | ~L1537 | 基于音频分析的口型同步 |
| 口型同步 | `animateChunk()` | ~L1423 | 口型逐帧动画 |
| 口型同步 | `getFormant()` | ~L1399 | 共振峰频率检测 |
| 表情控制 | chunk animation expression logic | ~L1494-L1524 | 基于 chunk 状态设置表情 |
| 表情控制 | render chunk update (expression+blend) | ~L1656-L1677 | 表情与 BlendShape 混合更新 |
| 场景 | `loadGaussScene()` | ~L342 | 高斯溅射场景加载 |
| 姿态 | `setNaturalPose()` | ~L449 | 自然姿态重置 |
| 渲染 | `animate()` | ~L2120 | 主渲染循环 |
| VMC | `getVMCBlendData()` | ~L2064 | 获取 BlendShape 数据（表情列表） |

### 1.3 许可差异（关键）

| 组件 | 许可 | 可否在 MIT 项目中使用？ |
|------|------|------------------------|
| `@pixiv/three-vrm` (npm) | **MIT** | ✅ 可直接使用 |
| `@pixiv/three-vrm-animation` (npm) | **MIT** | ✅ 可直接使用 |
| `three` (npm) | **MIT** | ✅ 可直接使用 |
| super-agent-party `vrm.js` | **AGPL-3.0** | ❌ 不可复制源码 |
| super-agent-party 整体 | **AGPL-3.0** | ❌ |
| agent-diva 整体 | **MIT** | — |

> **核心原则**：可使用 MIT 许可的 `@pixiv/three-vrm` 库，基于其官方文档/示例自主实现 VRM 集成，不复制 super-agent-party 的任何 AGPL-3.0 代码。

---

## 2. VRM 渲染架构深度分析

### 2.1 技术栈分层

```
┌────────────────────────────────────────────────────┐
│              VRM 渲染栈（MIT 许可）                 │
│                                                    │
│  ┌──────────────────────────────────────────────┐  │
│  │  应用层：DivaVrmAvatar.vue (Vue 3 组件)       │  │
│  │  - Three.js Scene/Camera/Renderer 管理        │  │
│  │  - VRM 模型加载与生命周期                      │  │
│  │  - 表情 / 口型同步 / 动画控制                   │  │
│  │  - 响应式 Watch (messages → expression)       │  │
│  └──────────────────┬───────────────────────────┘  │
│                     │                              │
│  ┌──────────────────┴───────────────────────────┐  │
│  │  VRM 核心层：@pixiv/three-vrm                │  │
│  │  - VRMLoaderPlugin (GLTFLoader 扩展)         │  │
│  │  - VRMUtils (模型优化)                        │  │
│  │  - VRMHumanoid (骨骼映射)                     │  │
│  │  - VRMExpressionManager (表情 BlendShape)     │  │
│  │  - VRMLookAt (视线跟踪)                       │  │
│  │  - VRMSpringBone (物理模拟)                    │  │
│  └──────────────────┬───────────────────────────┘  │
│                     │                              │
│  ┌──────────────────┴───────────────────────────┐  │
│  │  动画层：@pixiv/three-vrm-animation          │  │
│  │  - VRMAnimationLoaderPlugin                  │  │
│  │  - createVRMAnimationClip()                  │  │
│  │  - .vrma 格式解析                             │  │
│  └──────────────────┬───────────────────────────┘  │
│                     │                              │
│  ┌──────────────────┴───────────────────────────┐  │
│  │  渲染引擎层：Three.js                         │  │
│  │  - WebGLRenderer / Scene / Camera             │  │
│  │  - GLTFLoader (glTF 2.0 解析)                 │  │
│  │  - AnimationMixer (动画混合)                   │  │
│  │  - OrbitControls (相机控制)                    │  │
│  │  - AudioListener / AudioAnalyser (音频分析)    │  │
│  └──────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────┘
```

### 2.2 VRM 模型格式

```
VRM 文件结构 (.vrm = glTF 2.0 + 扩展):

.vrm (zip-like container)
├── glTF JSON (scene, nodes, meshes, materials)
├── VRMC_vrm (VRM 扩展元数据)
│   ├── meta (名称, 作者, 许可)
│   ├── humanoid (骨骼映射: head, spine, arms, legs...)
│   ├── firstPerson (第一人称设置)
│   ├── lookAt (视线跟踪)
│   ├── expressions (表情 BlendShape)
│   │   ├── preset: happy, sad, angry, surprised, relaxed, neutral
│   │   ├── custom: aa, ih, ou, ee, oh (口型)
│   │   └── custom: blink, blink_l, blink_r (眨眼)
│   └── springBone (物理骨骼)
├── textures/ (贴图)
├── meshes/ (网格数据)
└── animations/ (内嵌动画，可选)
```

### 2.3 VRM 0.x vs 1.0

| 特性 | VRM 0.x | VRM 1.0 |
|------|---------|---------|
| 规范状态 | 已废弃 | 当前标准 |
| MToon 材质 | v0.x MToon | v1.0 MToon |
| 表情 | BlendShapeGroup | Expression (更灵活) |
| `@pixiv/three-vrm` 版本 | v1.x (仅 VRM 0.x) | v3.x (VRM 1.0 支持) |
| 模型来源 | 旧版 VRoid Studio | 新版 VRoid Studio |

> **推荐**：使用 `@pixiv/three-vrm` v3.x 支持 VRM 1.0，VRoid Studio 默认导出格式。

### 2.4 渲染管线

```
1. 初始化 Three.js 环境
   ├── Scene + Camera (PerspectiveCamera)
   ├── WebGLRenderer (antialias, transparent background)
   ├── Lighting (AmbientLight + DirectionalLight + shadow)
   └── OrbitControls (可选，旋转/缩放)

2. 加载 VRM 模型
   ├── GLTFLoader.register(VRMLoaderPlugin)
   ├── loader.loadAsync('/vrm/model.vrm')
   ├── VRMUtils.removeUnnecessaryJoints(gltf.scene)  // 优化
   └── vrm = gltf.userData.vrm
       ├── vrm.scene → 添加到 Three.js Scene
       ├── vrm.expressionManager → 表情控制
       ├── vrm.lookAt → 视线跟踪
       └── vrm.springBone → 物理模拟

3. 渲染循环 (requestAnimationFrame)
   ├── clock.getDelta()
   ├── vrm.update(delta)  ← 更新 springBone, lookAt, expression
   ├── mixer.update(delta) ← 更新动画
   └── renderer.render(scene, camera)
```

---

## 3. 表情系统

### 3.1 VRM 预设表情

```typescript
// @pixiv/three-vrm 支持的预设表情 (ExpressionPreset)
type ExpressionPreset =
  | 'happy'      // 开心
  | 'sad'        // 悲伤
  | 'angry'      // 生气
  | 'surprised'  // 惊讶
  | 'relaxed'    // 放松
  | 'neutral'    // 中性 (默认)
  | 'aa' | 'ih' | 'ou' | 'ee' | 'oh'  // 口型 (AIUEO)
  | 'blink' | 'blinkLeft' | 'blinkRight'  // 眨眼
```

### 3.2 表情权重控制

```typescript
// 设置单一表情
vrm.expressionManager.setValue('happy', 1.0)

// 混合多个表情
vrm.expressionManager.setValue('happy', 0.6)
vrm.expressionManager.setValue('surprised', 0.4)

// 重置所有表情
vrm.expressionManager.resetValues()

// 获取可用表情列表
const expressions = vrm.expressionManager.expressions
```

---

## 4. 口型同步架构

### 4.1 Audio Analyser 方案

```
TTS 音频播放
  │
  ├── AudioContext.createMediaElementSource(audio)
  ├── AnalyserNode (FFT size 256)
  │
  └── requestAnimationFrame 循环
      ├── analyser.getByteTimeDomainData(dataArray)
      ├── 计算音量 (RMS)
      └── 映射到口型:
          ├── 高音量 → 'aa' (大开口) = 1.0
          ├── 中音量 → 'ou' (圆唇) = 0.6
          ├── 低音量 → 'ih' (微开) = 0.3
          └── 静音   → 重置所有口型 = 0.0
```

### 4.2 简化方案（推荐 MVP）

> 不实现音频分析，而是基于 TTS 播放状态做简单的口型开合：

```typescript
// 播放 TTS 时：周期性切换口型
let mouthTimer = 0
function updateMouth(delta: number) {
  if (!isSpeaking) {
    vrm.expressionManager.setValue('aa', 0)
    return
  }
  mouthTimer += delta
  const value = 0.5 + 0.5 * Math.sin(mouthTimer * 8)  // 正弦波形
  vrm.expressionManager.setValue('aa', value)
}
```

---

## 5. 动画系统

### 5.1 VRM Animation (.vrma)

```typescript
// 加载 VRM 动画
import { VRMAnimationLoaderPlugin, createVRMAnimationClip } from '@pixiv/three-vrm-animation'

const animLoader = new GLTFLoader()
animLoader.register(parser => new VRMAnimationLoaderPlugin(parser))

const animGltf = await animLoader.loadAsync('/vrm/animations/greeting.vrma')
const animData = animGltf.userData.vrmAnimations[0]
const clip = createVRMAnimationClip(animData, vrm)

// 播放
const action = mixer.clipAction(clip)
action.setLoop(THREE.LoopOnce, 1)
action.clampWhenFinished = true
action.play()
```

### 5.2 可用动画预设

> **源路径**: `super-agent-party/vrm/animations/` 目录下共 11 个 .vrma 文件

| 文件 | 动作 | 路径 |
|------|------|------|
| `greeting.vrma` | 挥手打招呼 | [`super-agent-party/vrm/animations/greeting.vrma`](../../../../../super-agent-party/vrm/animations/greeting.vrma) |
| `peace_sign.vrma` | 比耶 | [`super-agent-party/vrm/animations/peace_sign.vrma`](../../../../../super-agent-party/vrm/animations/peace_sign.vrma) |
| `shoot.vrma` | 射击手势 | [`super-agent-party/vrm/animations/shoot.vrma`](../../../../../super-agent-party/vrm/animations/shoot.vrma) |
| `spin.vrma` | 旋转 | [`super-agent-party/vrm/animations/spin.vrma`](../../../../../super-agent-party/vrm/animations/spin.vrma) |
| `stretch.vrma` | 伸展 | [`super-agent-party/vrm/animations/stretch.vrma`](../../../../../super-agent-party/vrm/animations/stretch.vrma) |
| `squat.vrma` | 下蹲 | [`super-agent-party/vrm/animations/squat.vrma`](../../../../../super-agent-party/vrm/animations/squat.vrma) |
| `scratch_head.vrma` | 挠头 | [`super-agent-party/vrm/animations/scratch_head.vrma`](../../../../../super-agent-party/vrm/animations/scratch_head.vrma) |
| `akimbo.vrma` | 叉腰 | [`super-agent-party/vrm/animations/akimbo.vrma`](../../../../../super-agent-party/vrm/animations/akimbo.vrma) |
| `play_fingers.vrma` | 玩手指 | [`super-agent-party/vrm/animations/play_fingers.vrma`](../../../../../super-agent-party/vrm/animations/play_fingers.vrma) |
| `show_full_body.vrma` | 展示全身 | [`super-agent-party/vrm/animations/show_full_body.vrma`](../../../../../super-agent-party/vrm/animations/show_full_body.vrma) |
| `model_pose.vrma` | 模特姿势 | [`super-agent-party/vrm/animations/model_pose.vrma`](../../../../../super-agent-party/vrm/animations/model_pose.vrma) |

> 这些 .vrma 文件为 VRM Animation 格式，通过 `@pixiv/three-vrm-animation` 的 `VRMAnimationLoaderPlugin` + `createVRMAnimationClip()` 加载播放（参考 `vrm.js` L1000 `loadVRMAAnimation()` 的 API 用法，但不可复制其代码）。

---

## 6. 整合到 Agent Diva 的目标架构

### 6.1 VRM 模块结构

```
agent-diva-gui/src/features/diva-pet-vrm/
├── index.ts                          # 统一导出
├── composables/
│   ├── useVrmModel.ts               # VRM 模型加载与管理
│   ├── useVrmExpression.ts          # 表情控制（基于 messages 情绪推断）
│   ├── useVrmMouthSync.ts           # 口型同步
│   └── useVrmAnimation.ts           # 动画播放
├── services/
│   ├── vrm-loader.ts                # VRM 文件加载 (Tauri IPC)
│   └── vrm-config.ts                # VRM 配置管理
├── components/
│   ├── DivaVrmAvatar.vue            # VRM 角色渲染 (Three.js Canvas)
│   ├── DivaVrmControlPanel.vue      # 表情/动画/场景控制
│   └── DivaVrmModelManager.vue      # 模型管理 UI
├── types.ts
└── shims-vrm.d.ts                   # 类型补丁

public/vrm/
├── models/                          # 默认 VRM 模型
│   └── alice.vrm
└── animations/                      # 默认 VRM 动画
    ├── greeting.vrma
    └── ...
```

### 6.2 与现有 Chat Session 集成

与 Live2D 方案完全相同——VRM 组件也是 `messages[]` 的消费者：

```typescript
// DivaVrmAvatar.vue
const props = defineProps<{ messages: Message[]; isTyping: boolean }>()

// 根据最新 Agent 回复驱动 VRM 表情
const latestReply = computed(() =>
  props.messages.filter(m => m.role === 'agent').at(-1)
)

watch(latestReply, (reply) => {
  if (!reply || !vrm.value) return
  // 文本情绪分析 → VRM 表情
  const mood = detectMood(reply.content)
  vrm.value.expressionManager.setValue(mood, 1.0)
})
```

### 6.3 与 Live2D 的共存方案

```
config.json:
{
  "pet": {
    "enabled": true,
    "renderer": "vrm",          // "vrm" | "live2d" | "none"
    "vrm": {
      "modelPath": "vrm/models/alice.vrm",
      "cameraDistance": 2.0,
      "enableShadows": true
    },
    "live2d": {
      "modelPath": "live2d_resource/default/mao_pro.model3.json",
      "scale": 0.72
    }
  }
}
```
