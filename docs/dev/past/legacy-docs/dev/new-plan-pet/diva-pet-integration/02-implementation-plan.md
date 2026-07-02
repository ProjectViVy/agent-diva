# 02 - 实现方案讲解

> Diva Pet 模块（Live2D + ASR + TTS）的分阶段实现方案

---

## 1. 总览

```
Phase 1 ──────── Phase 2 ──────── Phase 3 ──────── Phase 4 ──────── Phase 5
基础设施        Live2D 渲染       语音能力          集成 UI            优化打磨
(1-2d)          (3-4d)            (1-2d)            (2-3d)            (1-2d)
```

---

## 2. Phase 1：基础设施搭建（1-2 天）

### 2.1 依赖安装

```bash
cd agent-diva-gui
pnpm add live2dcubismcore@1.0.2
pnpm add pixi.js@6.5.10
pnpm add pixi-live2d-display@0.4.0
```

> **注意**：`pixi-live2d-display` 仅用于其 DisplayObject 类型定义和基础抽象，实际渲染使用原生 WebGL + Cubism 5 Framework。

### 2.2 Vendor 脚本迁移

从 AniPet 复制 Cubism 5 Framework 文件到 `public/live2d/`：

```
源:  AniPet/apps/desktop/src/vendor/cubism5-framework/
目标: agent-diva-gui/public/live2d/cubism5/framework/

需复制的文件（按 AniPet vendor 目录结构）：
├── live2dcubismframework.js      ← 核心框架入口
├── cubismdefaultparameterid.js   ← 默认参数ID常量
├── cubismmodelsettingjson.js     ← .model3.json 解析
├── cubismusermodel.js            ← 用户模型基类 (CubismUserModel)
├── cubismmotion.js               ← 动作数据
├── cubismmotionqueuemanager.js   ← 动作队列管理
├── cubismmotionqueueentry.js     ← 动作队列条目
├── cubismmotionmanager.js        ← 动作管理器
├── cubismmotionjson.js           ← .motion3.json 解析
├── cubismmotioninternal.js       ← 动作内部实现
├── cubismexpressionmotion.js     ← 表情动作
├── cubismexpressionmotionmanager.js ← 表情动作管理
├── cubismbreath.js               ← 呼吸效果 (effect/)
├── cubismeyeblink.js             ← 眨眼效果 (effect/)
├── cubismmodel.js                ← 模型数据 (model/)
├── cubismmoc.js                  ← .moc3 解析 (model/)
├── cubismmodeluserdata.js        ← 模型用户数据 (model/)
├── cubismmodeluserdatajson.js    ← 用户数据JSON解析 (model/)
├── rendering/cubismrenderer_webgl.js      ← WebGL 渲染器
├── rendering/cubismrenderer.js            ← 抽象渲染器
├── rendering/cubismshader_webgl.js        ← WebGL 着色器管理
├── rendering/cubismrendertarget_webgl.js  ← WebGL 渲染目标
├── rendering/cubismoffscreenmanager.js    ← 离屏渲染管理
├── rendering/cubismclippingmanager.js     ← 裁剪管理
├── math/cubismmatrix44.js         ← 矩阵工具
├── id/cubismid.js                 ← Cubism ID 系统
├── ic/cubismmodelsettingjson.js   ← （同 cubismmodelsettingjson）
├── physics/cubismphysics.js       ← 物理引擎
├── physics/cubismphysicsjson.js   ← 物理配置解析
├── physics/cubismphysicsinternal.js ← 物理内部实现
├── utils/cubismstring.js          ← 字符串工具
├── utils/cubismjson.js            ← JSON 工具
├── utils/cubismdebug.js           ← 调试工具
└── type/csmrectf.js               ← RectF 类型

着色器资源:
源:  AniPet 的 LIVE2D_SHADER_BASE_PATH 配置 (/live2d/cubism5/shaders/)
目标: agent-diva-gui/public/live2d/cubism5/shaders/
注意: cubism5-model.ts L29-L32 中定义路径为 "/live2d/cubism5/shaders/"
```

> **说明**: AniPet 的 vendor 目录结构比文档前面列出的更完整。实际复制时应包含 `rendering/`、`motion/`、`model/`、`physics/`、`math/`、`id/`、`utils/`、`type/` 等子目录。并非所有文件都必须（如 `utils/cubismdebug.js` 可省略），但 `rendering/cubismshader_webgl.js` 和 `rendering/cubismrenderer_webgl.js` 是核心依赖。

### 2.3 着色器资源

确保 `public/live2d/cubism5/shaders/` 包含 WebGL 着色器：
- `vertex_shader.glsl`（或等效 JS 字符串）
- `fragment_shader.glsl`

### 2.4 模块目录结构

```bash
mkdir -p agent-diva-gui/src/features/diva-pet/{composables,services,components,live2d}
touch agent-diva-gui/src/features/diva-pet/index.ts
```

---

## 3. Phase 2：Live2D 渲染（3-4 天）

### 3.1 步骤 2.1 — cubism5-core.ts 迁移

> **参考源文件**: [`AniPet/apps/desktop/src/components/live2d-avatar/cubism5-core.ts`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/cubism5-core.ts):1-379

将 AniPet 的 `cubism5-core.ts`（379行）直接复制到 `features/diva-pet/live2d/cubism5-core.ts`，仅调整路径：

```typescript
// AniPet 源文件 L6-7 中的路径常量
const CUBISM5_CORE_SCRIPT_SRC = "/live2d/cubism5/live2dcubismcore.js";
const CUBISM5_CORE_READY_TIMEOUT_MS = 15000;  // L7: 15s 超时

// L29-L32: cubism5-model.ts 中使用的着色器路径常量（在不同文件）
const LIVE2D_SHADER_BASE_PATH = new URL("/live2d/cubism5/shaders/", window.location.origin).toString();
```

**关键函数对照**：
| 函数 | 行号 | 作用 |
|------|------|------|
| `ensureCubism5CoreReady()` | L322-379 | 入口：动态加载 Wasm + 兼容补丁 + Framework 启动 |
| `waitForCubismCoreRuntimeReady()` | L273-297 | 轮询等待 Cubism Core Wasm 就绪（40ms 间隔，15s 超时） |
| `patchCubism5RuntimeCompatibility()` | L246-271 | 旧模型兼容补丁（blendConstants + modelShape） |
| `ensureCubismFrameworkStarted()` | L299-310 | 启动 CubismFramework |
| `resetCubismFrameworkState()` | L312-320 | 重置 Framework 状态（模型切换时用） |
| `getRecentCubism5Logs()` | L36-38 | 获取最近日志（调试） |

**改动点**：
- 调整 import 路径指向新的 vendor 位置（`../../vendor/cubism5-framework/`）
- 保持 `@ts-nocheck`（Cubism SDK 无类型定义，见 L1）
- `CUBISM5_CORE_SCRIPT_SRC` 路径保持不变（`/live2d/cubism5/live2dcubismcore.js`，在 `public/` 下）

### 3.2 步骤 2.2 — cubism5-model.ts 迁移

> **参考源文件**: [`AniPet/apps/desktop/src/components/live2d-avatar/cubism5-model.ts`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/cubism5-model.ts):1-1524

将 AniPet 的 `cubism5-model.ts`（1524行）直接复制到 `features/diva-pet/live2d/cubism5-model.ts`，仅调整内部 import 路径。

**核心类 `AniPetCubism5Model`（L470-1517）重命名为 `DivaPetCubism5Model`**。

**关键结构索引**：
| 符号 | 行号 | 说明 |
|------|------|------|
| `Cubism5ModelLoadOptions` interface | L383-390 | 模型加载选项 |
| `AniPetCubism5Model` class | L470-1517 | 核心模型类（继承 `CubismUserModel`） |
| `buildBundleFileIndex()` | L391-416 | 构建 bundle 文件索引 |
| `readCoreMocDiagnostics()` | L418-468 | 读取 .moc3 诊断信息 |
| `decodeTextureImage()` | L324-381 | 纹理解码（WebGL mipmap + 各向异性过滤） |
| `findMotionGroupByKeywords()` | L294-311 | 按关键词匹配动作组 |
| `createCubism5Model()` | L1518-1524 | 工厂函数：创建模型实例 |
| `Cubism5ModelRuntime` type | L1524 | 导出类型别名 |

```typescript
// 关键 API 保持不变：
export interface Cubism5ModelLoadOptions {
  bundle: LoadedLive2dModelBundle;
  gl: WebGLRenderingContext;
  modelLabel: string;
  viewportHeight: number;
  viewportWidth: number;
}

export async function createCubism5Model(options): Promise<DivaPetCubism5Model>

// 模型实例方法：
class DivaPetCubism5Model {
  render(deltaSeconds: number): void
  setViewportOptions(scale, offsetX, offsetY): void
  setExpression(name: string): boolean
  clearExpression(): void
  setDesiredMotionGroup(group: string | null): void
  getExpressionNames(): string[]
  getMotionGroupNames(): string[]
  setDragTarget(localX, localY, width, height): void
  stopDragging(): void
  hitTestAt(localX, localY, width, height): Cubism5HitArea
  isAnimating(): boolean
  dispose(): void
}
```

### 3.3 步骤 2.3 — DivaPetAvatar.vue（核心组件）

> **参考源文件**: [`AniPet/apps/desktop/src/components/live2d-avatar/Live2DAvatarRenderer.tsx`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/Live2DAvatarRenderer.tsx):1-1373

将 React 组件 `Live2DAvatarRenderer.tsx`（1373行）改写为 Vue 3 组件。

**参考源文件关键常量**（L33-L43）：
```typescript
const CANVAS_HEIGHT = 460;           // L33
const CANVAS_WIDTH = 360;            // L34
const QUALITY_SCALE_FACTOR = 1.5;    // L36: 高清渲染倍率
const DRAG_THRESHOLD_PX = 8;         // L37: 拖拽阈值
```
**可见区域追踪**（L39-L43）：使用 `readPixels` 实现像素级 hit test。
**自适应帧率**：空闲 30fps / 活跃 60fps（AniPet 实现于渲染循环内）。

**改写对照表**：

| React 模式 | Vue 3 等价 |
|------------|-----------|
| `useState(initial)` | `ref(initial)` |
| `useEffect(fn, [deps])` | `watch(deps, fn)` / `onMounted(fn)` / `onUnmounted(fn)` |
| `useRef(initial)` | `ref(initial)` (DOM: `template ref`) |
| `useMemo(fn, deps)` | `computed(fn)` |
| `useCallback(fn, deps)` | 普通函数（Vue 自动缓存） |
| `props.xxx` | `defineProps<{ xxx }>()` → `props.xxx` |
| `onXxx` callback props | `defineEmits<{ xxx: [...] }>()` |
| JSX (`.tsx`) | `<template>` (`.vue`) + `<script setup>` |

**组件 Props**：

```typescript
interface Props {
  model: Live2dModelOption;         // 模型路径
  scale?: number;                    // 缩放 (default: 0.72)
  offsetX?: number;                  // X 偏移
  offsetY?: number;                  // Y 偏移
  emotion?: string;                  // 当前情绪 "happy" | "neutral" | "shy"
  live2dExpression?: string | null;  // LLM 指定的表情
  live2dMotionGroup?: string | null; // LLM 指定的动作组
}
```

**组件 Emits**：

```typescript
const emit = defineEmits<{
  (e: 'load-start'): void
  (e: 'load-success'): void
  (e: 'load-error', error: Error): void
  (e: 'head-activate'): void        // 点击头部
  (e: 'body-activate'): void        // 点击身体
  (e: 'drag-start'): void
  (e: 'drag-end'): void
}>()
```

**Canvas 生命周期管理**（Vue 3 关键）：

```typescript
// 使用 template ref 获取 Canvas DOM
const canvasRef = ref<HTMLCanvasElement | null>(null)

onMounted(() => {
  const canvas = canvasRef.value!
  const gl = canvas.getContext('webgl', { alpha: true, premultipliedAlpha: true })
  // ... 初始化
})

onUnmounted(() => {
  modelRef.value?.dispose()      // 释放 Live2D 资源
  cancelAnimationFrame(rafId)    // 停止渲染循环
  gl?.getExtension('WEBGL_lose_context')?.loseContext()
})
```

**关键渲染循环**（与框架无关，直接使用 rAF）：

```typescript
function startRenderLoop() {
  let lastTime = performance.now()
  function frame(now: number) {
    const delta = Math.min((now - lastTime) / 1000, 0.05)
    lastTime = now
    model.value?.render(delta)
    rafId = requestAnimationFrame(frame)
  }
  rafId = requestAnimationFrame(frame)
}
```

### 3.4 步骤 2.4 — 模型加载流程

```typescript
// 模型文件由 Tauri Rust 后端提供（类似 AniPet 的 customization-api）

// 前端调用：
import { invoke } from '@tauri-apps/api/core'

interface Live2dModelFile {
  relativePath: string
  base64Data: string     // 文件内容 base64
  contentType: string    // MIME type
}

interface Live2dModelBundle {
  modelRelativePath: string
  files: Live2dModelFile[]
}

// Rust 端（需添加到 commands.rs）：
#[tauri::command]
async fn load_live2d_model(path: String) -> Result<Live2dModelBundle, String> {
    // 递归读取模型目录所有文件 → base64 编码
}

#[tauri::command]
async fn list_live2d_models() -> Result<Vec<String>, String> {
    // 扫描 live2d_resource/ 目录下所有 .model3.json
}
```

---

## 4. Phase 3：语音能力（1-2 天）

### 4.1 步骤 3.1 — tts-service.ts 迁移

> **参考源文件**: [`AniPet/apps/desktop/src/features/voice/tts-service.ts`](../../../../../AniPet/apps/desktop/src/features/voice/tts-service.ts):1-1048

**直接复制** AniPet 的 `tts-service.ts`（1048行）到 `features/diva-pet/services/tts-service.ts`。该类完全框架无关，仅依赖 `fetch` 和 `Audio` API。

**源文件关键结构**：
| 符号 | 行号 | 说明 |
|------|------|------|
| `TTSVoiceConfig` interface | L8-17 | TTS 配置结构 |
| `TTSRequestError` class | L58-78 | 统一错误类型 |
| `PROVIDER_DEFAULTS` | L80-89 | Provider 默认 baseUrl/model |
| `DEFAULT_RETRYABLE_STATUSES` | L97 | 可重试的 HTTP 状态码 |

唯一需调整的 import：

```typescript
// 原 AniPet 依赖 (L1)
import { readVoiceFile } from "../customization/customization-api";

// agent-diva 改写为 Tauri IPC 调用
async function readVoiceFile(path: string): Promise<VoiceFileData> {
  return invoke('read_voice_file', { path })
}
```

### 4.2 步骤 3.2 — useVoiceInput.ts (Vue Composable)

> **参考源文件**: [`AniPet/apps/desktop/src/features/voice/use-voice-input.ts`](../../../../../AniPet/apps/desktop/src/features/voice/use-voice-input.ts):1-402

将 React Hook `useVoiceInput`（402行）改写为 Vue Composable。

**源文件关键点**：
- L3: `VOICE_INPUT_RESTART_DELAY_MS = 420` — 自动重连延迟
- L72-78: `getSpeechRecognitionConstructor()` — 浏览器兼容性检测
- L29-L41: `SpeechRecognitionLike` interface — 完整 Web Speech API 类型定义
- L54-L59: `UseVoiceInputOptions` — Hook 参数接口

```typescript
// features/diva-pet/composables/useVoiceInput.ts

export function useVoiceInput(options: {
  language?: string          // default: "zh-CN"
  onRecognizedText: (text: string) => void
}) {
  const isListening = ref(false)
  const isSupported = computed(() => 
    !!(window.SpeechRecognition || window.webkitSpeechRecognition)
  )
  const error = ref<string | null>(null)

  const start = () => { /* ... */ }
  const stop = () => { /* ... */ }
  const toggle = () => { /* ... */ }

  onUnmounted(() => {
    recognition?.abort()
  })

  return { isListening, isSupported, error, start, stop, toggle }
}
```

### 4.3 步骤 3.3 — useVoicePlayer.ts (Vue Composable)

```typescript
// features/diva-pet/composables/useVoicePlayer.ts

export function useVoicePlayer(config: Ref<TTSVoiceConfig>) {
  const isPlaying = ref(false)

  // 监听 Agent 回复完成事件
  onMounted(() => {
    unlisten = await listen<string>('agent-response-complete', (event) => {
      const text = extractReplyText(event.payload)
      speak(text)
    })
  })

  async function speak(text: string) {
    isPlaying.value = true
    await ttsService.speakText(text, config.value)
    isPlaying.value = false
  }

  function stop() {
    ttsService.stopPlayback()
    isPlaying.value = false
  }

  onUnmounted(() => { unlisten?.() })

  return { isPlaying, speak, stop }
}
```

---

## 5. Phase 4：集成 UI（2-3 天）

### 5.1 步骤 4.1 — DivaPetView.vue（Session 感知组件）

新建主视图组件，作为桌宠的容器页面。**关键设计：接收与 ChatView 完全相同的 `messages` props，emit 相同的 `send` 事件，自动共享当前 session**。

```vue
<script setup lang="ts">
import { computed } from 'vue'
import type { Message } from '../../App.vue'
import DivaPetAvatar from './DivaPetAvatar.vue'
import DivaPetBubble from './DivaPetBubble.vue'
import DivaPetVoicePanel from './DivaPetVoicePanel.vue'

interface Props {
  messages: Message[]        // ← 来自 App.vue，与 ChatView 共享
  isTyping: boolean          // ← 来自 App.vue，与 ChatView 共享
}

const props = defineProps<Props>()

const emit = defineEmits<{
  (e: 'send', content: string): void  // ← 与 ChatView 完全相同的 emit
  (e: 'clear'): void
  (e: 'stop'): void
}>()

// ── 从共享 messages 中实时提取当前 session 的对话内容 ──

/** 最新一条 Agent 回复（用于气泡和 TTS） */
const latestAgentReply = computed(() => {
  for (let i = props.messages.length - 1; i >= 0; i--) {
    if (props.messages[i].role === 'agent') return props.messages[i]
  }
  return null
})

/** 气泡显示的文本 */
const bubbleText = computed(() => {
  const reply = latestAgentReply.value
  if (!reply) return ''
  if (reply.isStreaming && !reply.content) return '...'
  return reply.content
})

/** 气泡是否可见 */
const bubbleVisible = computed(() => {
  return !!latestAgentReply.value && (
    latestAgentReply.value.isStreaming ||
    Date.now() - (latestAgentReply.value.timestamp ?? 0) < 10000  // 最近 10s
  )
})

/** 根据 Agent 回复内容推断 Live2D 表情 */
const currentEmotion = computed(() => {
  const reply = latestAgentReply.value
  if (reply?.emotion) return reply.emotion
  if (props.isTyping) return 'neutral'
  return 'happy'
})

// ── 发送消息（自动使用当前 session） ──
function handleSend(text: string) {
  if (!text.trim()) return
  emit('send', text)  // → NormalMode → App.sendMessage() → 同一 currentChatId
}
</script>

<template>
  <div class="diva-pet-view">
    <!-- Live2D 角色 -->
    <DivaPetAvatar
      :model="currentModel"
      :scale="0.72"
      :emotion="currentEmotion"
      @head-activate="handleSend('你好呀~')"
    />

    <!-- 对话气泡：实时显示当前 session 的最新 Agent 回复 -->
    <DivaPetBubble
      :visible="bubbleVisible"
      :text="bubbleText"
      :is-streaming="latestAgentReply?.isStreaming ?? false"
    />

    <!-- 语音控制栏 -->
    <DivaPetVoicePanel
      :is-listening="isListening"
      :is-speaking="isSpeaking"
      :voice-config="voiceConfig"
      @toggle-mic="toggleMic"
      @send-text="handleSend"
      @update-config="updateVoiceConfig"
    />
  </div>
</template>
```

**Session 行为验证**：

| 操作 | 预期行为 | 原理 |
|------|----------|------|
| 在 DivaPet 输入文字 "你好" | 消息出现在 ChatView 和 DivaPet 气泡中 | `messages[]` 是共享的 ref |
| 在 ChatView 输入 "你好" | DivaPet 气泡同步显示回复 | 同上 |
| DivaPet ASR 语音输入 | 消息发送到同一 `currentChatId` | emit('send') 走同一通道 |
| 切换 Tab: Chat → DivaPet → Settings | messages 不受影响 | `v-if` 只销毁视图，不销毁 App.vue 状态 |
| 点击 "清空对话" | ChatView 和 DivaPet 都变为空白 | `clearMessages()` 重置 messages |
| 加载历史 session | ChatView 和 DivaPet 同步恢复历史 | `loadSession()` 替换 messages |

### 5.2 步骤 4.2 — NormalMode.vue 修改

在侧边栏添加 "Diva Pet" 入口。**关键：DivaPetView 使用与 ChatView 相同的 props/emits 接口**：

```vue
<!-- 侧边栏新增 -->
<button
  class="w-full text-left px-3 py-2 rounded-lg text-sm font-medium flex items-center"
  :class="sidebarItemClass('pet')"
  @click="navigateTo('pet')"
>
  <span class="flex items-center space-x-2">
    <Smile :size="16" :class="sidebarIconClass('pet', 'text-violet-500')" />
    <span>Diva Pet</span>
  </span>
</button>

<!-- main content 区新增 -->
<div v-if="activeTab === 'pet'" class="h-full">
  <DivaPetView
    :messages="messages"        ← 与 ChatView 相同的 props!
    :is-typing="isTyping"      ← 与 ChatView 相同的 props!
    @send="(text) => emit('send', text)"  ← 与 ChatView 相同的 emits!
    @clear="emit('clear')"
    @stop="emit('stop')"
  />
</div>
```

**侧边栏状态管理扩展**：

```typescript
// NormalMode.vue - 新增 pet 在 tab 切换逻辑中
type SidebarSection = 'chat' | 'pet' | 'settings' | 'console' | 'neuro' | 'cron'

const activeTab = ref<'chat' | 'pet' | 'settings'>('chat')
// 当 DivaPet 入口被点击时：activeTab.value = 'pet'

function navigateTo(target: SidebarSection) {
  if (target === 'pet') {
    activeTab.value = 'pet'
    activeMenu.value = null
  } else if (target === 'chat') {
    activeTab.value = 'chat'
    activeMenu.value = null
  } else if (target === 'settings') {
    activeTab.value = 'settings'
    activeMenu.value = null
  } else {
    activeMenu.value = target
    activeTab.value = 'chat'  // menu 不替换 tab
  }
}
```

### 5.3 步骤 4.3 — 模型管理 UI

在 Settings 面板新增 "Live2D Model" 子页面：

```
SettingsView
└── Live2dSettings.vue
    ├── 模型列表（当前可用模型）
    ├── 导入新模型（.model3.json 文件夹）
    ├── 缩放/位置调节滑块
    └── 预览窗口
```

---

## 6. Phase 5：优化打磨（1-2 天）

### 6.1 性能优化

- 非激活窗口暂停 Live2D 渲染（`Page Visibility API`）
- 空闲时降帧至 15fps（而非 30fps）
- 模型纹理懒加载
- WebGL context 复用

### 6.2 配置持久化

```json
// config.json 新增 section
{
  "pet": {
    "enabled": true,
    "live2d": {
      "modelPath": "live2d_resource/default/mao_pro.model3.json",
      "scale": 0.72,
      "offsetX": 0.0,
      "offsetY": 0.0
    },
    "voice": {
      "enabled": true,
      "provider": "siliconflow",
      "apiKey": "",
      "baseUrl": "https://api.siliconflow.cn/v1",
      "model": "FunAudioLLM/CosyVoice2-0.5B",
      "referenceVoice": "",
      "referenceText": "",
      "speed": 1.0
    }
  }
}
```

### 6.3 错误处理

- Live2D 加载失败 → 显示静态占位图 + 错误提示
- WebGL 不可用 → 降级为静态角色图
- ASR 权限被拒 → 禁用语音按钮 + 提示引导
- TTS API 超时 → 降级到浏览器 TTS → 静默失败

---

## 7. 文件变更清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `agent-diva-gui/package.json` | 修改 | 添加 npm 依赖 |
| `agent-diva-gui/public/live2d/` | 新增 | Cubism 5 运行时和着色器 |
| `agent-diva-gui/src/features/diva-pet/` | 新增 | 完整新模块 |
| `agent-diva-gui/src/components/NormalMode.vue` | 修改 | 添加侧边栏入口 |
| `agent-diva-gui/src/components/DivaPetView.vue` | 新增 | 桌宠主视图 |
| `agent-diva-gui/src/App.vue` | 修改 | 集成 pet 配置和事件 |
| `agent-diva-gui/src-tauri/src/commands.rs` | 修改 | 添加 Live2D/语音命令 |
| `agent-diva-gui/src-tauri/Cargo.toml` | 修改 | 可能需要文件操作依赖 |
| `agent-diva-gui/src/locales/zh.ts` | 修改 | 添加中文翻译 |
| `agent-diva-gui/src/locales/en.ts` | 修改 | 添加英文翻译 |
