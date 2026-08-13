# 01 - 架构设计探索

> 调研 AniPet 的 Live2D + ASR-TTS 能力整合到 Agent Diva 的技术架构方案

---

## 1. 现有系统架构分析

### 1.1 Agent Diva 架构概览

```
┌────────────────────────────────────────────────────────┐
│                    Agent Diva                          │
│                                                        │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────┐    │
│  │   CLI    │  │   TUI    │  │   GUI (Tauri 2)    │    │
│  │ (Rust)   │  │ (Rust)   │  │ Vue 3 + Rust       │    │
│  └────┬─────┘  └────┬─────┘  └────────┬──────────┘    │
│       │             │                │                 │
│  ┌────┴─────────────┴────────────────┴─────────────┐   │
│  │              agent-diva-core                     │   │
│  │  配置 / 会话 / 记忆 / Cron / 事件总线            │   │
│  └──────────────────────┬──────────────────────────┘   │
│       ┌─────────────────┼──────────────────┐           │
│  ┌────┴────┐  ┌─────────┴──────┐  ┌───────┴──────┐    │
│  │ Providers│  │  Agent Loop    │  │  Channels    │    │
│  │ (LLM/ASR)│  │  + Tools       │  │  (10+ 通道)  │    │
│  └─────────┘  └────────────────┘  └──────────────┘    │
└────────────────────────────────────────────────────────┘
```

**GUI 前端技术栈**：
| 层级 | 技术 |
|------|------|
| 桌面壳 | Tauri 2 |
| 框架 | Vue 3 (Composition API) |
| 语言 | TypeScript 5.6 |
| 构建 | Vite 6 |
| 样式 | TailwindCSS 3 |
| 国际化 | vue-i18n 9 |
| 图标 | lucide-vue-next |

**GUI 组件树**：
```
App.vue
├── WelcomeWizard.vue          (首次引导)
└── NormalMode.vue             (主布局)
    ├── Sidebar                (左侧导航：Chat/Settings/Console/Neuro/Cron)
    └── Main Content
        ├── ChatView.vue       (对话界面)
        ├── SettingsView.vue   (配置面板)
        │   ├── ProvidersSettings.vue
        │   ├── ChannelsSettings.vue
        │   ├── GeneralSettings.vue
        │   ├── NetworkSettings.vue
        │   ├── LanguageSettings.vue
        │   ├── McpSettings.vue
        │   ├── SkillsSettings.vue
        │   └── AboutSettings.vue
        └── CronTaskManagementView.vue
```

**Rust 后端 (src-tauri)**：
```
src-tauri/
├── main.rs              # Tauri 入口
├── lib.rs               # Tauri Plugin 注册
├── commands.rs          # Tauri IPC 命令 (2586行)
│   ├── send_message     # 发送消息到 Agent
│   ├── stop_generation  # 停止流式生成
│   ├── load_config      # 加载配置
│   ├── save_config      # 保存配置
│   ├── get_sessions     # 获取会话列表
│   ├── get_session_history  # 加载历史消息
│   ├── delete_session   # 删除会话
│   ├── check_health     # 健康检查
│   ├── start_background_stream  # 启动后台流
│   └── ... (共 ~30+ Commands)
├── app_state.rs         # Agent 运行状态管理
├── gateway_status.rs    # Gateway 状态
├── embedded_server.rs   # 内嵌 HTTP Server (端口 3000)
├── process_utils.rs     # 进程管理工具
├── tray.rs              # 系统托盘
└── shutdown_manager.rs  # 优雅关闭
```

**关键依赖 crates**：
- `agent-diva-core` — 共享配置、记忆/会话、事件总线
- `agent-diva-manager` — Gateway 管理 API
- `agent-diva-cli` — CLI 运行时
- `agent-diva-neuron` — 神经元节点（LLM 推理）
- `agent-diva-providers` — LLM Provider 抽象

---

### 1.2 AniPet 架构概览

```
┌─────────────────────────────────────────────────────────┐
│                      AniPet                             │
│                                                         │
│  ┌──────────────────────────────────────────────────┐   │
│  │          Desktop App (Tauri 2 + React 18)        │   │
│  │                                                   │   │
│  │  ┌─────────────┐ ┌────────────┐ ┌─────────────┐  │   │
│  │  │ Live2D      │ │ Chat Panel │ │ Task Center │  │   │
│  │  │ Renderer    │ │ (对话+Agent)│ │ (Agent任务)  │  │   │
│  │  │ (WebGL)     │ │            │ │             │  │   │
│  │  └──────┬──────┘ └─────┬──────┘ └──────┬──────┘  │   │
│  │         │              │               │          │   │
│  │  ┌──────┴──────────────┴───────────────┴──────┐   │   │
│  │  │          App Runtime (事件总线)             │   │   │
│  │  │  DialogueAgent / MemoryService / Voice      │   │   │
│  │  └──────────────────────┬──────────────────────┘   │   │
│  └─────────────────────────┼──────────────────────────┘   │
│                            │                              │
│  ┌─────────────────────────┴──────────────────────────┐   │
│  │          Rust Host (src-tauri)                     │   │
│  │  窗口/文件/配置/自定义资源                         │   │
│  └────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

**前端技术栈**：
| 层级 | 技术 |
|------|------|
| 桌面壳 | Tauri 2 |
| 框架 | React 18 |
| 语言 | TypeScript 5.6 |
| 构建 | Vite 5 |
| WebGL | PixiJS 6 (pixi-live2d-display) |
| Live2D | Cubism SDK 5 (live2dcubismcore 1.0.2) |
| ASR | Web Speech API (浏览器原生) |
| TTS | SiliconFlow CosyVoice2 / OpenAI TTS / Web SpeechSynthesis |

**关键源文件参考路径**：
| 模块 | AniPet 路径 | 行数 |
|------|-----------|------|
| Cubism5 运行时加载 | [`AniPet/apps/desktop/src/components/live2d-avatar/cubism5-core.ts`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/cubism5-core.ts):1-379 | 379 |
| Live2D 模型管理 | [`AniPet/apps/desktop/src/components/live2d-avatar/cubism5-model.ts`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/cubism5-model.ts):1-1524 | 1524 |
| React 渲染组件 | [`AniPet/apps/desktop/src/components/live2d-avatar/Live2DAvatarRenderer.tsx`](../../../../../AniPet/apps/desktop/src/components/live2d-avatar/Live2DAvatarRenderer.tsx):1-1373 | 1373 |
| TTS 核心服务 | [`AniPet/apps/desktop/src/features/voice/tts-service.ts`](../../../../../AniPet/apps/desktop/src/features/voice/tts-service.ts):1-1048 | 1048 |
| ASR Hook | [`AniPet/apps/desktop/src/features/voice/use-voice-input.ts`](../../../../../AniPet/apps/desktop/src/features/voice/use-voice-input.ts):1-402 | 402 |
| 语音播放 Hook | [`AniPet/apps/desktop/src/features/voice/use-voice-player.ts`](../../../../../AniPet/apps/desktop/src/features/voice/use-voice-player.ts):1-204 | 204 |
| 角色渲染外壳 | [`AniPet/apps/desktop/src/components/avatar-renderer/AvatarRenderer.tsx`](../../../../../AniPet/apps/desktop/src/components/avatar-renderer/AvatarRenderer.tsx):1-187 | 187 |
| 桌宠主壳 | [`AniPet/apps/desktop/src/components/pet-shell/PetShell.tsx`](../../../../../AniPet/apps/desktop/src/components/pet-shell/PetShell.tsx):1-474 | 474 |
| Cubism5 Vendor | [`AniPet/apps/desktop/src/vendor/cubism5-framework/`](../../../../../AniPet/apps/desktop/src/vendor/cubism5-framework/) | 目录 |
| Cubism5 Core Wasm | [`AniPet/vendor/official-live2dcubismcore.min.js`](../../../../../AniPet/vendor/official-live2dcubismcore.min.js) | 二进制 |

---

## 2. Live2D 渲染架构深度分析

### 2.1 技术栈分层

```
┌──────────────────────────────────────────────────────┐
│                  Live2D 渲染栈                       │
│                                                      │
│  ┌────────────────────────────────────────────────┐  │
│  │  应用层：Live2DAvatarRenderer.tsx              │  │
│  │  - React 组件，管理 Canvas + WebGL 生命周期     │  │
│  │  - Pointer 事件处理（拖拽、点击）              │  │
│  │  - 自适应帧率（空闲 30fps / 活跃 60fps）      │  │
│  │  - 模型自动适配（auto-fit）                    │  │
│  │  - 可见区域追踪（pixel-level hit test）        │  │
│  └────────────────────┬───────────────────────────┘  │
│                       │                              │
│  ┌────────────────────┴───────────────────────────┐  │
│  │  模型层：cubism5-model.ts (1524 行)             │  │
│  │  - AniPetCubism5Model extends CubismUserModel   │  │
│  │  - 模型加载（.model3.json → .moc3 + 贴图）     │  │
│  │  - 表情系统（expression manager）              │  │
│  │  - 动作组管理（motion groups + priority）      │  │
│  │  - 物理模拟（physics + pose）                  │  │
│  │  - 呼吸效果（breath + eye blink）              │  │
│  │  - 拖拽变形（drag manager）                    │  │
│  │  - 视口变换（viewport matrix）                  │  │
│  │  - GPU 纹理管理（WebGL + mipmap）              │  │
│  └────────────────────┬───────────────────────────┘  │
│                       │                              │
│  ┌────────────────────┴───────────────────────────┐  │
│  │  运行时层：cubism5-core.ts (379 行)            │  │
│  │  - Cubism 5 Core 动态加载（<script> 注入）     │  │
│  │  - CubismFramework 生命周期管理                 │  │
│  │  - 运行时兼容性补丁（旧模型格式适配）          │  │
│  │  - 15s 超时保护                                 │  │
│  └────────────────────┬───────────────────────────┘  │
│                       │                              │
│  ┌────────────────────┴───────────────────────────┐  │
│  │  依赖层                                        │  │
│  │  ├── live2dcubismcore (npm, 1.0.2)             │  │
│  │  │   └── WebAssembly Cubism 5 核心运行时       │  │
│  │  ├── pixi-live2d-display (npm, 0.4.0)          │  │
│  │  │   └── PixiJS → Live2D 桥接（实际使用有限）  │  │
│  │  ├── pixi.js (npm, 6.5.10)                     │  │
│  │  │   └── WebGL 渲染框架（仅依赖其 DisplayObject)│  │
│  │  └── vendor/cubism5-framework/  (官方 SDK)     │  │
│  │      ├── live2dcubismframework.js              │  │
│  │      ├── cubismmodelsettingjson.js             │  │
│  │      ├── cubismusermodel.js                    │  │
│  │      ├── cubismmotion.js / cubismmotionqueue*  │  │
│  │      ├── cubismbreath.js / cubismeyeblink.js   │  │
│  │      ├── cubismoffscreenmanager.js             │  │
│  │      └── cubismshader_webgl.js                 │  │
│  └────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────┘
```

### 2.2 模型资源结构

```
live2d_resource/
└── {model-name}/
    ├── {model-name}.model3.json   # 模型描述文件（入口）
    ├── {model-name}.moc3          # Cubism 5 二进制模型数据
    ├── textures/                  # 贴图目录
    │   ├── texture_00.png
    │   └── ...
    ├── motions/                   # 动作数据 (.motion3.json)
    │   ├── Idle/
    │   ├── Tap/
    │   └── ...
    ├── expressions/               # 表情数据 (.exp3.json)
    │   ├── happy.exp3.json
    │   ├── sad.exp3.json
    │   └── ...
    └── physics/                   # 物理配置（可选）
        └── {model-name}.physics3.json
```

### 2.3 渲染管线

```
1. ensureCubism5CoreReady()
   ├── 动态加载 live2dcubismcore.js
   ├── 等待 Wasm 运行时就绪（15s 超时）
   └── 启动 CubismFramework

2. loadLive2dModelBundle(modelPath)
   └── 通过 Tauri IPC 读取模型文件目录 → 返回 base64 bundle

3. createCubism5Model(bundle, gl)
   ├── 解析 .model3.json → CubismModelSettingJson
   ├── 加载 .moc3 → CubismModel (WebGL buffer)
   ├── 并行解码贴图 → GPU 纹理（mipmap + 各向异性过滤）
   ├── 加载表情 → expression map
   ├── 预加载动作 → motion map
   ├── 设置物理/姿态/呼吸/眨眼
   └── 返回 Cubism5ModelRuntime

4. render(deltaSeconds) — 每帧调用
   ├── updateModel(deltaSeconds)
   │   ├── motionManager.update()
   │   ├── expressionManager.update()
   │   ├── eyeBlink.update()
   │   ├── breath.update()
   │   ├── physics.evaluate()
   │   ├── pose.update()
   │   └── dragManager.update()
   ├── applyViewportTransform()
   ├── gl.clear() + gl.viewport()
   ├── renderer.drawModel()
   └── 自适应帧率控制（idle 30fps / active 60fps）
```

---

## 3. ASR（语音识别）架构分析

### 3.1 实现机制

> **参考源文件**：[`AniPet/apps/desktop/src/features/voice/use-voice-input.ts`](../../../../../AniPet/apps/desktop/src/features/voice/use-voice-input.ts):1-402（React Hook，402 行）

```typescript
// use-voice-input.ts — React Hook (AniPet参考: L72 getSpeechRecognitionConstructor, L80-L402 主逻辑)
// 底层：浏览器 Web Speech API

const recognition = new (window.SpeechRecognition || window.webkitSpeechRecognition)();

recognition.continuous = false;       // 单句模式
recognition.interimResults = true;    // 开启中间结果
recognition.lang = "zh-CN";           // 中文识别
recognition.maxAlternatives = 1;

recognition.onresult = (event) => {
  // 收集 final transcript → 回调 onRecognizedText
};

recognition.onerror = (event) => {
  // 错误分类处理：
  // "no-speech"     → 静默忽略，自动重启
  // "audio-capture" → 无麦克风，禁用
  // "not-allowed"   → 权限被拒，禁用
  // "network"       → 网络不可用
};

// 自动重连：每次识别完成后 420ms 重新 start
// 暂停机制：TTS 播放期间自动暂停识别
```

**关键特性**：
- 零外部依赖，完全浏览器原生
- Windows 需要 WebView2（Tauri 2 已包含）
- 识别结果通过系统语音服务处理（Windows 使用 Cortana 引擎）
- 不支持离线识别（依赖系统语音服务）

---

## 4. TTS（语音合成）架构分析

### 4.1 多 Provider 降级链

```
用户文本
  │
  ├── Provider: siliconflow (CosyVoice2)
  │   ├── 有 referenceVoice?
  │   │   ├── YES → 声音克隆模式（上传参考音频 → 克隆语音）
  │   │   │   ├── 成功 → 播放克隆语音
  │   │   │   └── 失败 → 降级到标准 TTS
  │   │   └── NO → 标准 TTS 模式
  │   └── 失败 → 降级到 browser TTS
  │
  ├── Provider: openai
  │   ├── 标准 OpenAI TTS API
  │   └── 失败 → 降级到 browser TTS
  │
  └── Provider: browser
      └── window.speechSynthesis.speak(utterance)
```

### 4.2 TTSService 类架构

> **参考源文件**：[`AniPet/apps/desktop/src/features/voice/tts-service.ts`](../../../../../AniPet/apps/desktop/src/features/voice/tts-service.ts):1-1048（框架无关纯 TS，可直接复制）

```
TTSService (单例)
├── synthesize(text, config)          # 核心合成方法
│   ├── synthesizeWithVoiceCloning()  # CosyVoice2 声音克隆
│   ├── synthesizeWithAPI()           # 标准 API TTS
│   └── synthesizeWithBrowser()       # 浏览器兜底
├── speakText(text, config)           # 合成 + 自动播放
├── playAudio(url)                    # 音频播放控制
├── stopPlayback()                    # 停止播放
├── prepareVoiceClone(config)         # 预加载克隆语音（缓存）
│   ├── ensureCloneVoiceUri()
│   ├── findExistingCloneVoiceUri()   # 查找已上传的语音
│   └── uploadCloneVoice()            # 上传参考音频
└── 工具方法
    ├── resolveProviderConfig()       # 解析 Provider 配置
    └── performRequest()              # HTTP 请求（重试+超时）
```

### 4.3 TTS 配置结构

```typescript
interface TTSVoiceConfig {
  enabled: boolean;           // 是否启用语音
  provider: string;           // "browser" | "openai" | "siliconflow" | "local"
  apiKey: string | null;      // API Key
  baseUrl: string;            // API Base URL
  model: string | null;       // 模型 ID
  referenceVoice: string | null;  // 参考音频路径（用于声音克隆）
  referenceText: string | null;   // 参考文本
  speed: number;              // 语速 (0.5 - 2.0)
  volume: number;             // 音量 (0.0 - 1.0)
}
```

---

## 5. 整合架构方案

### 5.1 核心设计原则

1. **前端优先**：ASR/TTS/Live2D 均为前端能力，不引入不必要的 Rust 复杂度
2. **模块隔离**：新功能作为独立 `features/diva-pet/` 模块，不影响现有 Chat/Settings
3. **可开关**：通过配置开关，用户可选择是否启用桌宠功能
4. **渐进增强**：先 MVP（Live2D + TTS），再迭代 ASR

### 5.2 目标架构

```
agent-diva-gui/src/
├── features/
│   └── diva-pet/                        # 新增模块
│       ├── index.ts                     # 统一导出
│       ├── composables/
│       │   ├── useVoiceInput.ts         # 语音输入 (Vue Composable)
│       │   └── useVoicePlayer.ts        # 语音播放 (Vue Composable)
│       ├── services/
│       │   └── tts-service.ts           # TTS 核心服务（框架无关）
│       ├── components/
│       │   ├── DivaPetAvatar.vue        # Live2D 角色渲染（WebGL Canvas）
│       │   ├── DivaPetBubble.vue        # 对话气泡
│       │   ├── DivaPetVoicePanel.vue    # 语音控制面板
│       │   └── DivaPetSettings.vue      # 设置面板
│       ├── live2d/
│       │   ├── cubism5-core.ts          # Cubism 5 运行时管理
│       │   ├── cubism5-model.ts         # 模型加载与管理
│       │   └── types.ts                 # Live2D 类型定义
│       └── types.ts                     # 模块类型定义
├── components/
│   ├── NormalMode.vue                   # 修改：添加 "Diva Pet" 侧边栏
│   └── DivaPetView.vue                  # 新增：桌宠主视图
└── App.vue                              # 修改：集成 pet 配置

public/
└── live2d/
    ├── cubism5/
    │   ├── live2dcubismcore.js          # Cubism 5 Wasm 运行时
    │   ├── shaders/                     # WebGL 着色器
    │   └── framework/                   # Cubism 5 Framework
    └── models/                          # 默认 Live2D 模型
        └── default/
            └── *.model3.json, *.moc3, textures/
```

### 5.3 Session 共享机制（关键设计）

**当前 Agent Diva 的 session 架构**（从 `App.vue` 代码确认）：

```
App.vue — 单一数据源 (Single Source of Truth)
│
├── messages: Ref<Message[]>          ← 所有消息
├── currentChatId: Ref<string>        ← 当前会话 ID
├── currentSessionKey: Ref<string>    ← 当前 session key
├── isTyping: Ref<boolean>            ← 流式状态
│
├── sendMessage(content)              ← 唯一发送入口
│   ├── invoke("send_message", ...)   → Tauri IPC
│   └── 流式事件监听
│       ├── "agent-response-delta"    → messages[last].content += data
│       ├── "agent-reasoning-delta"   → messages[last].reasoning += data
│       ├── "agent-tool-start"        → messages.push(tool msg)
│       ├── "agent-tool-end"          → messages[tool].status = success
│       ├── "agent-response-complete" → messages[last].isStreaming = false
│       └── "agent-error"             → messages.push(error msg)
│
├── loadSession(key)                  ← 加载历史 session
├── deleteSession(key)                ← 删除 session
└── clearMessages()                   ← 新建 session
    │
    └── NormalMode.vue (props 透传 + events 冒泡)
        ├── props: { messages, isTyping, sessions, ... }
        ├── emits: { send, clear, stop, load-session, delete-session }
        │
        ├── ChatView.vue (v-if="activeTab === 'chat'")
        │   ├── props: { messages, isTyping }
        │   ├── emits: { send, clear, stop }
        │   └── 输入框 → emit('send', text) → NormalMode → App.sendMessage(text)
        │
        └── DivaPetView.vue (v-if="activeTab === 'pet'")  ← 新增
            ├── props: { messages, isTyping }              ← 相同 props!
            ├── emits: { send, clear, stop }               ← 相同 emits!
            ├── DivaPetAvatar  → 从 messages 计算最新 agent 回复 → 气泡
            ├── 文字输入框     → emit('send', text) → 同一通道
            └── ASR 语音输入   → emit('send', text) → 同一通道
```

**关键结论**：Diva Pet 与 Chat 天然共享 session，因为：

1. **`messages[]` 的 owner 是 `App.vue`**，不是任何一个子组件。切换 Tab 时 ChatView 被 `v-if` 销毁，但 messages 保留在 App.vue 中。
2. **发送路径完全一致**：无论从 ChatView 还是 DivaPetView 发送，最终都调用 `App.vue.sendMessage()`，使用相同的 `currentChatId` / `currentSessionKey`。
3. **流式事件监听在 `App.vue` 层级**（`listen("agent-response-delta", ...)`），所有下游视图自动同步。
4. **DivaPetView 不需要修改任何 Rust 后端或 IPC 命令**——它只是 `messages[]` 的另一个消费者。

### 5.4 与 Agent Loop 的集成点

```
用户输入（文字/ASR 语音）
       │
       ▼
┌──────────────────────────────────────────────────┐
│               App.vue.sendMessage()               │
│  (唯一入口，使用 currentChatId / currentSessionKey) │
└──────────────────────┬───────────────────────────┘
                       │ Tauri IPC
                       ▼
┌──────────────────────────────────────────────────┐
│              Agent Loop (Rust 后端)               │
│  invoke("send_message", { message, channel,       │
│    chatId: currentChatId, ... })                  │
└──────────────────────┬───────────────────────────┘
                       │ streaming events
                       ▼
┌──────────────────────────────────────────────────┐
│              App.vue 事件监听                      │
│  messages[] ← 实时更新                            │
└──────┬───────────────────────┬───────────────────┘
       │                       │
       ▼                       ▼
┌──────────────┐     ┌──────────────────┐
│  ChatView    │     │  DivaPetView     │
│  (文本列表)   │     │  ├─ 气泡 (最新)  │
│              │     │  ├─ Live2D 表情  │
│              │     │  └─ TTS 播报     │
└──────────────┘     └──────────────────┘
  共享同一个 messages[] — 同一 session，同一对话！
```

**DivaPetView 内部数据流**：

```
messages (props)
  │
  ├── computed: latestAgentReply
  │   └── messages.filter(m => m.role === 'agent').at(-1)
  │       ├── .content → DivaPetBubble (气泡文本)
  │       ├── .isStreaming → 气泡闪烁动画
  │       └── .isThinking → Live2D "思考" 表情
  │
  ├── computed: latestUserMessage
  │   └── messages.filter(m => m.role === 'user').at(-1)
  │
  └── computed: currentEmotion
      └── 基于 latestAgentReply 的语气 → Live2D 表情选择

ASR 输入 / 文字输入
  └── emit('send', text)
      └── NormalMode emit('send', text, attachments)
          └── App.sendMessage(text, attachments)
              └── 使用同一 currentChatId → 同一 session!
```

### 5.5 数据流（完整端到端）

```
[用户语音] → ASR (Web Speech API) → recognizedText
    → DivaPetView emit('send', text)
    → NormalMode emit('send', text, attachments)
    → App.sendMessage(text, attachments)
        → messages.push({ role: 'user', content: text })       ← 同时写入 messages[]
        → Tauri IPC invoke("send_message", {
            message: text, channel: "gui", chatId: currentChatId  ← 同一 session!
          })
    → Agent Loop 处理
        → streaming events (Tauri listen):
        ├── "agent-response-delta"
        │   → messages[last].content += data
        │   → ChatView 文本增量更新 + DivaPet 气泡实时流式
        ├── "agent-reasoning-delta"
        │   → messages[last].reasoning += data
        │   → ChatView thinking 展开 + DivaPet Live2D "思考" 表情
        ├── "agent-tool-start"
        │   → messages.push({ role: 'tool', status: 'running' })
        │   → ChatView 工具卡片 + DivaPet Live2D "工作中" 动作
        ├── "agent-tool-end"
        │   → messages[tool].status = 'success'/'error'
        │   → ChatView 工具结果 + DivaPet 表情反馈
        └── "agent-response-complete"
            → messages[last].isStreaming = false
            → ChatView 停止闪烁 + DivaPet TTS 播放完整回复
```
**注意**：以上所有 `messages[]` 操作都在 `App.vue` 中完成，ChatView 和 DivaPetView 仅作为响应式消费者。

---

## 6. 关键决策记录

| 决策 | 选项 | 选择 | 理由 |
|------|------|------|------|
| ASR/TTS 打包为 Rust crate? | Yes / No | **No** | 均为纯前端能力，Rust crate 增加不必要复杂度 |
| Live2D 渲染框架 | PixiJS 6 / 原生 WebGL | **原生 WebGL** | 减少依赖，PixiJS 6 已过时，AniPet 实际使用有限 |
| 前端模块位置 | 独立 npm 包 / features/ | **features/diva-pet/** | 避免 monorepo 复杂度，后续可提取为 npm 包 |
| 配置存储 | config.json / 独立文件 | **config.json pet section** | 统一配置管理 |
| ASR Provider | Web Speech / Whisper / 云端 | **Web Speech (MVP)** | 零成本，后续可扩展 |
| TTS Provider | CosyVoice2 / OpenAI / Browser | **CosyVoice2 + Browser fallback** | 中文音色好，全降级链 |
