# 05 - 兼容性与迁移分析

> Diva Pet 模块对现有系统的影响分析

---

## 1. 兼容性矩阵

### 1.1 平台兼容性

| 平台 | Live2D | ASR (Web Speech) | TTS (Browser) | TTS (API) |
|------|--------|-----------------|---------------|-----------|
| **Windows 10** | ✅ | ✅ (中文) | ✅ | ✅ |
| **Windows 11** | ✅ | ✅ (中文) | ✅ | ✅ |
| **Windows 7/8** | ✅ | ⚠️ (英文) | ✅ | ✅ |
| **macOS 12+** | ✅ | ✅ | ✅ | ✅ |
| **Linux (X11)** | ✅ | ⚠️ (无中文) | ✅ | ✅ |
| **Linux (Wayland)** | ✅ | ⚠️ | ✅ | ✅ |

> **注**：Web Speech API 在不同平台的中文支持差异较大。Windows 10+ 和 macOS 内置中文语音识别，Linux 通常不支持。建议始终提供文本输入作为主要交互方式。

### 1.2 WebView2 版本要求

| 功能 | 最低 WebView2 版本 |
|------|-------------------|
| WebGL 1.0 | 89+ (Edge 89) |
| SpeechRecognition | 89+ |
| SpeechSynthesis | 89+ |
| createImageBitmap | 89+ |
| AbortController | 89+ |

> Tauri 2 在 Windows 上默认使用 WebView2 Evergreen（自动更新），以上 API 均满足。

### 1.3 浏览器环境兼容性

| 功能 | Chromium | Firefox | Safari |
|------|----------|---------|--------|
| WebGL | ✅ | ✅ | ✅ |
| SpeechRecognition | ✅ | ❌ | ❌ |
| SpeechSynthesis | ✅ | ✅ | ✅ |
| Audio API | ✅ | ✅ | ✅ |

> **关键差异**：ASR 功能仅在 Chromium 内核可用（Tauri Windows/Linux 默认 WebView2 = Chromium），Firefox 和 Safari 不支持 `SpeechRecognition`。

---

## 2. 对现有功能的兼容性影响

### 2.1 对 Chat 功能的影响

| 影响点 | 程度 | 说明 |
|--------|------|------|
| 消息处理 | 零影响 | 复用现有 `sendMessage()` / `agent-response-*` 事件流 |
| 消息显示 | 零影响 | DivaPet 读取流事件但不修改 ChatView 数据 |
| 会话管理 | 零影响 | 不涉及会话创建/删除/切换 |
| 性能 | 低影响 | Live2D 渲染独立于 Chat，不同窗口可独立运行 |

### 2.2 对 Settings 功能的影响

| 影响点 | 程度 | 说明 |
|--------|------|------|
| config.json 结构 | 低影响 | 新增 `pet` section，不修改现有字段 |
| Settings 面板 | 低影响 | 新增 `Live2dSettings.vue` 子页面 |
| API Key 管理 | 低影响 | 独立管理 TTS API Key |

### 2.3 对其他模块的影响

| 模块 | 影响 | 说明 |
|------|------|------|
| agent-diva-core | 零影响 | 不修改核心 crate |
| agent-diva-providers | 零影响 | 不新增 Provider |
| agent-diva-channels | 零影响 | Diva Pet 是 GUI 专属功能 |
| Tauri commands | 低影响 | 新增 3-5 个命令（`pet_*` 前缀） |

---

## 3. 依赖冲突分析

### 3.1 NPM 依赖检查

```
现有 agent-diva-gui 依赖：
  vue: ^3.5.13
  vite: ^6.0.3
  typescript: ~5.6.2
  tailwindcss: ^3.4.17

AniPet 使用的依赖：
  pixi.js: 6.5.10
  live2dcubismcore: 1.0.2
  pixi-live2d-display: 0.4.0

冲突检查：
  ✅ vue - 无冲突（agent-diva 不依赖 react）
  ✅ vite - 版本兼容（5.x vs 6.x，API 小差异）
  ✅ typescript - 版本兼容
  ✅ tailwindcss - 无冲突
  ✅ pixi.js - 新依赖，无冲突
  ⚠️ pixi.js 6 依赖某些较旧的 Web API，需验证
```

### 3.2 潜在的 pixi.js v6 问题

PixiJS v6 使用了部分在最新 Vite 6 + TypeScript 5.6 下可能产生警告的 API：

```typescript
// 可能的类型不兼容
import * as PIXI from 'pixi.js';  // v6 使用 namespace export

// v6 的 TypeScript 定义可能缺少某些类型声明
// 解决：添加 declare module 补丁
```

**缓解方案**：
1. 优先使用原生 WebGL 渲染（绕过 PixiJS 大部分 API）
2. 仅使用 PixiJS 的 DisplayObject 和基础类型
3. 为 PixiJS v6 添加类型补丁文件

---

## 4. Tauri 配置兼容性

### 4.1 tauri.conf.json

```json
// 需新增的配置项
{
  "app": {
    "windows": [
      {
        "label": "pet",
        "title": "Diva Pet",
        "decorations": false,    // 无边框窗口（可选）
        "transparent": true,     // 透明背景
        "alwaysOnTop": false,
        "skipTaskbar": true
      }
    ]
  },
  "bundle": {
    "resources": [
      "public/live2d/**",       // 打包 Live2D 资源
      "live2d_resource/**"       // 打包默认模型
    ]
  },
  "plugins": {
    "store": {}                  // 已有的 tauri-plugin-store
  }
}
```

### 4.2 Rust 依赖

```toml
# src-tauri/Cargo.toml 新增
[dependencies]
# 文件操作（读取模型目录）
walkdir = "2"                   # 目录遍历
base64 = "0.22"                 # base64 编码（已有 serde_json，无需新增）
```

**注意**：agent-diva-gui 的 Cargo.toml 已有 `serde`, `serde_json`, `tokio` 等，无需大量新增。

---

## 5. 向后兼容性

### 5.1 配置向后兼容

```json
// 旧版 config.json（无 pet section）→ 完全兼容
{
  "providers": { /* ... */ },
  "agents": { /* ... */ }
  // pet 字段缺失：默认禁用桌宠功能
}

// 新版 config.json（有 pet section）
{
  "providers": { /* ... */ },
  "agents": { /* ... */ },
  "pet": {
    "enabled": false  // 可显式禁用
  }
}
```

### 5.2 构建向后兼容

- `pnpm build` 不依赖 Live2D 资源时仍正常构建
- public/live2d/ 目录缺失时，仅 DivaPet 页面不可用，其他页面不受影响
- Live2D npm 依赖通过 `optionalDependencies` 或动态 import 实现可选加载

### 5.3 运行时向后兼容

```typescript
// 优雅降级模式
const isLive2dAvailable = computed(() => {
  try {
    // 检查 WebGL 支持
    const canvas = document.createElement('canvas')
    return !!canvas.getContext('webgl')
  } catch {
    return false
  }
})

// 模板中：
<DivaPetAvatar v-if="isLive2dAvailable" />
<StaticCharacterImage v-else />
```

---

## 6. 迁移路径

### 6.1 从 AniPet 的代码迁移路径（精确源→目标对照）

```
AniPet 源文件                                                         → agent-diva 目标位置

Vendor 脚本（直接复制，无需适配）:
┌ apps/desktop/src/vendor/cubism5-framework/                          → public/live2d/cubism5/framework/
│   ├── live2dcubismframework.js       (核心框架入口)                    ├── live2dcubismframework.js
│   ├── cubismdefaultparameterid.js    (默认参数)                       ├── cubismdefaultparameterid.js
│   ├── cubismmodelsettingjson.js      (.model3.json 解析)             ├── cubismmodelsettingjson.js
│   ├── cubismusermodel.js             (用户模型基类)                   ├── cubismusermodel.js
│   ├── cubismmotion.js                (动作)                          ├── cubismmotion.js
│   ├── cubismmotionqueuemanager.js    (动作队列)                      ├── cubismmotionqueuemanager.js
│   ├── rendering/cubismrenderer_webgl.js (WebGL 渲染核心)             ├── rendering/cubismrenderer_webgl.js
│   ├── rendering/cubismshader_webgl.js   (着色器管理)                 ├── rendering/cubismshader_webgl.js
│   ├── model/cubismmoc.js             (.moc3 二进制解析)               ├── model/cubismmoc.js
│   ├── effect/cubismbreath.js         (呼吸效果)                      ├── effect/cubismbreath.js
│   ├── effect/cubismeyeblink.js       (眨眼效果)                      ├── effect/cubismeyeblink.js
│   ├── physics/cubismphysics.js       (物理引擎)                      ├── physics/cubismphysics.js
│   ├── math/cubismmatrix44.js         (矩阵工具)                      ├── math/cubismmatrix44.js
│   └── id/cubismid.js                 (ID系统)                        └── id/cubismid.js
├ vendor/official-live2dcubismcore.min.js (Cubism 5 Wasm 核心)         → public/live2d/cubism5/live2dcubismcore.js

核心逻辑类（直接复制 + 调整 import 路径）:
├ apps/desktop/src/components/live2d-avatar/cubism5-core.ts   (379行)  → features/diva-pet/live2d/cubism5-core.ts
│   ├── 关键函数: ensureCubism5CoreReady()      L322-379               │   调整: vendor import 路径
│   ├── 关键函数: patchCubism5RuntimeCompatibility() L246-271          │
│   └── 关键函数: waitForCubismCoreRuntimeReady()   L273-297           │
│                                                                       │
├ apps/desktop/src/components/live2d-avatar/cubism5-model.ts  (1524行) → features/diva-pet/live2d/cubism5-model.ts
│   ├── 核心类: AniPetCubism5Model                L470-1517            │   重命名: → DivaPetCubism5Model
│   ├── 工厂函数: createCubism5Model()            L1518-1524           │   保持不变
│   ├── 模型加载选项: Cubism5ModelLoadOptions     L383-390             │
│   ├── 纹理解码: decodeTextureImage()            L324-381             │
│   └── 着色器路径: LIVE2D_SHADER_BASE_PATH       L29-L32              │   路径保持一致
│                                                                       │
├ apps/desktop/src/features/voice/tts-service.ts          (1048行)     → features/diva-pet/services/tts-service.ts
│   ├── 配置接口: TTSVoiceConfig                   L8-17               │   唯一需改: readVoiceFile → Tauri IPC
│   ├── 错误类: TTSRequestError                    L58-78              │
│   └── Provider默认配置: PROVIDER_DEFAULTS        L80-L89              │
│                                                                       │
React Hooks（重写为 Vue 3 Composables）:
├ apps/desktop/src/components/live2d-avatar/Live2DAvatarRenderer.tsx (1373→Vue重写) → features/diva-pet/components/DivaPetAvatar.vue
│   ├── Canvas 常量: CANVAS_WIDTH=360, CANVAS_HEIGHT=460    L33-L34    │
│   ├── 质量倍率: QUALITY_SCALE_FACTOR=1.5                    L36      │
│   ├── Hit test 阈值常量                                   L39-L43    │
│   └── 自适应帧率逻辑 (30fps/60fps)                     render loop   │
│                                                                       │
├ apps/desktop/src/features/voice/use-voice-input.ts    (402→重写)     → features/diva-pet/composables/useVoiceInput.ts
│   ├── 延迟常量: VOICE_INPUT_RESTART_DELAY_MS=420         L3           │
│   ├── SpeechRecognition 类型定义                      L29-L53        │
│   └── getSpeechRecognitionConstructor()                L72-L78        │
│                                                                       │
├ apps/desktop/src/features/voice/use-voice-player.ts   (204→重写)     → features/diva-pet/composables/useVoicePlayer.ts
│   ├── speakWithLifecycle()                              L101-L129     │
│   └── 事件监听: PET_CHAT_REPLY_RECEIVED                L132-L157     │
│                                                                       │
React 组件（重写为 Vue 3 组件）:
├ apps/desktop/src/components/avatar-renderer/AvatarRenderer.tsx (187→重写) → features/diva-pet/components/ (合并到 DivaPetAvatar)
├ apps/desktop/src/components/pet-shell/PetShell.tsx      (474→重写)    → features/diva-pet/components/DivaPetView.vue
└ apps/desktop/src/components/pet-bubble/PetBubble.tsx    (React→重写)  → features/diva-pet/components/DivaPetBubble.vue

资源文件（直接复制）:
├ live2d_resource/default/mao_pro.model3.json                          → agent-diva-gui/live2d_resource/default/
├ live2d_resource/default/mao_pro.moc3                                 → agent-diva-gui/live2d_resource/default/
├ live2d_resource/default/mao_pro.4096/texture_00.png                  → agent-diva-gui/live2d_resource/default/textures/
├ live2d_resource/default/expressions/exp_01~08.exp3.json              → agent-diva-gui/live2d_resource/default/expressions/
├ live2d_resource/default/motions/mtn_01~04.motion3.json               → agent-diva-gui/live2d_resource/default/motions/
├ live2d_resource/default/mao_pro.physics3.json                        → agent-diva-gui/live2d_resource/default/
└ live2d_resource/default/mao_pro.pose3.json                           → agent-diva-gui/live2d_resource/default/
```

### 6.2 逐步迁移（推荐）

**Step 1**：复制 vendor 脚本和核心逻辑类（0 依赖变更）

```
public/live2d/cubism5/framework/*    ← 直接复制，无需适配
features/diva-pet/live2d/cubism5-*   ← 直接复制，调整 import 路径
features/diva-pet/services/tts-service.ts ← 直接复制
```

**Step 2**：改写 Vue 3 组件

```
DivaPetAvatar.vue   ← 参照 Live2DAvatarRenderer.tsx 逻辑重写
DivaPetView.vue     ← 参照 PetShell.tsx 结构重写（简化版）
```

**Step 3**：集成到现有布局

```
NormalMode.vue      ← 添加侧边栏入口
App.vue             ← 集成 pet 配置
```

**Step 4**：添加 Rust Tauri commands

```
commands.rs         ← 添加 pet_load_model, pet_list_models 等
```

---

## 7. 回滚方案

如果 Diva Pet 模块出现问题需要紧急回滚：

1. **移除侧边栏入口**（1 行代码注释）
2. **移除 `pet` section 从 config.json 默认模板**
3. **保留 `features/diva-pet/` 目录**（不影响其他功能）

回滚影响：零。不影响 Chat/Settings/Providers/Channels 任何现有功能。
