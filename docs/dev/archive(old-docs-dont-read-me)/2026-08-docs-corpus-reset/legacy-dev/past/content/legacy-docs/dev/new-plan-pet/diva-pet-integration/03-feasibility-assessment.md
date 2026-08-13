# 03 - 可行性评估

> 对整合 AniPet Live2D + ASR-TTS 到 Agent Diva 的全面可行性分析

---

## 1. 总体结论

| 维度 | 评估 | 说明 |
|------|------|------|
| **技术可行性** | ⭐⭐⭐⭐⭐ (95%) | 基础设施完全兼容，核心技术均可复用 |
| **架构契合度** | ⭐⭐⭐⭐ (85%) | Tauri 2 共享，仅需适配 Vue 3 框架差异 |
| **开发工作量** | ⭐⭐⭐ (中等) | 预计 7-10 人天（完整 MVP） |
| **维护风险** | ⭐⭐⭐⭐ (低) | 新模块独立，不耦合现有逻辑 |
| **用户体验增益** | ⭐⭐⭐⭐⭐ (高) | 桌宠角色 + 语音交互显著提升体验 |

**最终结论：✅ 高度可行，推荐启动 MVP 开发。**

---

## 2. 技术可行性逐项分析

### 2.1 桌面框架兼容性

| 检查项 | Agent Diva | AniPet | 兼容 |
|--------|-----------|--------|------|
| Tauri 版本 | v2 | v2 | ✅ |
| @tauri-apps/api 版本 | ^2 | 2.5.0 | ✅ |
| IPC 机制 | `invoke()` + `listen()` | `invoke()` + `listen()` | ✅ |
| 窗口管理 | Tauri Window API | Tauri Window API | ✅ |
| 文件系统访问 | Tauri Plugin | Tauri Plugin | ✅ |

**结论**：双发使用相同的 Tauri 2 运行时，IPC 调用完全兼容。

### 2.2 Live2D 渲染兼容性

| 检查项 | 依赖 | 状态 |
|--------|------|------|
| Cubism 5 Core Wasm | `live2dcubismcore` (1.0.2) | ✅ 可用 npm 安装 |
| WebGL 上下文 | 浏览器原生 | ✅ Tauri WebView2 完整支持 |
| Canvas 2D/WebGL | 浏览器原生 | ✅ |
| PixiJS 6 | `pixi.js` (6.5.10) | ⚠️ 版本较旧，但 Live2D 桥接依赖 |
| CubeScript Vendor | 官方 SDK 脚本 | ⚠️ 需手动复制到 public/ |

**Live2D 着色器兼容性**：Cubism 5 Framework 使用标准 WebGL 1.0 着色器，Tauri 2 的 WebView2 100% 支持。

**模型文件格式**：`.model3.json` + `.moc3` 为 Cubism 5 标准格式，与 Cubism 4（`.model.json` + `.moc`）不兼容。需确保模型为 Cubism 5 格式。

**结论**：✅ 可行，需手动集成 vendor 脚本和着色器资源。

### 2.3 ASR（语音识别）兼容性

| 检查项 | 技术 | 状态 |
|--------|------|------|
| SpeechRecognition API | 浏览器原生 | ✅ Chromium 内核 (WebView2) |
| 中文识别 | `lang="zh-CN"` | ✅ Windows 10+ 内置中文语音包 |
| 持续监听模式 | `continuous=false` (单句) | ✅ |
| 权限 | 麦克风权限请求 | ✅ Tauri 窗口支持权限弹窗 |
| 离线识别 | 依赖系统语音服务 | ⚠️ Windows 需联网（Cortana 引擎） |

**关键限制**：
- Windows 7/8 可能不支持中文语音识别
- Web Speech API 在不同 Windows 版本质量不一致
- 仅在 WebView2 运行时可用（Tauri 2 默认包含）

**结论**：✅ 可行，但建议标记为 "实验性功能"，并提供文本输入作为兜底。

### 2.4 TTS（语音合成）兼容性

| 检查项 | 技术 | 状态 |
|--------|------|------|
| SpeechSynthesis API | 浏览器原生 | ✅ |
| Fetch API (云端 TTS) | 浏览器原生 | ✅ |
| Audio API (播放) | 浏览器原生 | ✅ |
| CosyVoice2 API | SiliconFlow 云端 | ✅ HTTP/JSON |
| OpenAI TTS API | OpenAI 云端 | ✅ HTTP/JSON |

**结论**：✅ 完全可行，三种 TTS Provider 均有降级路径。

---

## 3. 框架差异风险分析

### 3.1 React → Vue 3 迁移复杂度矩阵

| 模块 | 行数 | React 依赖度 | 迁移难度 | 工时 |
|------|------|-------------|----------|------|
| `cubism5-core.ts` | 379 | 零依赖 | 直接复制 | 0.5h |
| `cubism5-model.ts` | 1524 | 零依赖 | 直接复制 | 0.5h |
| `tts-service.ts` | 1048 | 零依赖 | 直接复制 | 0.5h |
| `Live2DAvatarRenderer.tsx` | 1373 | 高（Hooks + JSX） | 全面重写 | 2d |
| `useVoiceInput.ts` | 402 | 中（Hooks） | 改写 Composable | 0.5d |
| `useVoicePlayer.ts` | 204 | 中（Hooks） | 改写 Composable | 0.5d |
| UI 组件 | ~500 | 高（React） | 全面重写 | 1d |

**总迁移工时：约 5 天**

### 3.2 风险缓解

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| Vue 3 Canvas 生命周期 bug | 中 | 中 | 先写最小可运行原型验证 |
| WebGL context 丢失 | 低 | 低 | 添加 context lost 事件处理 |
| PixiJS 6 与 Vite 6 不兼容 | 低 | 中 | 降级 Vite 或升级 PixiJS |
| Cubism 5 许可问题 | 低 | 高 | **发布前法务确认** |

---

## 4. 架构风险分析

### 4.1 耦合度评估

```
新模块与现有代码的耦合点：
┌────────────────────────────────────────┐
│ 耦合点              │ 耦合度  │ 风险  │
├────────────────────────────────────────┤
│ NormalMode.vue 侧边栏 │ 低      │ 低    │
│ App.vue sendMessage() │ 中      │ 低    │
│ config.json pet section│ 低      │ 低    │
│ Tauri commands.rs     │ 低      │ 低    │
│ 国际化文件            │ 低      │ 低    │
└────────────────────────────────────────┘
```

**结论**：新模块与现有代码的解耦度极高，所有变更均为增量式，不会破坏现有功能。

### 4.2 性能评估

| 场景 | Live2D 资源占用 | 评估 |
|------|----------------|------|
| 30fps 渲染（空闲） | GPU ~5%, CPU ~2% | ✅ 低 |
| 60fps 渲染（活跃） | GPU ~10%, CPU ~5% | ✅ 可接受 |
| 模型加载中 | 内存 +50-100MB (纹理) | ⚠️ 需异步加载 |
| TTS 合成中 | 网络请求 + 音频解码 | ✅ 异步 |

**自适应帧率策略**（AniPet 已验证）：
- 空闲（仅呼吸/眨眼）：30fps
- 活跃（动作/表情/拖拽）：60fps
- 非激活窗口：暂停渲染

### 4.3 包体积影响

| 新增依赖 | 大小 |
|----------|------|
| live2dcubismcore (Wasm) | ~1.5MB |
| Cubism5 Framework (JS) | ~200KB |
| pixi.js v6 | ~450KB minified |
| pixi-live2d-display | ~50KB |

**预计增加包体积：约 2-3MB**（gzip 后约 800KB）。可接受。

---

## 5. 关于"ASR-TTS 打包为 Rust crate"的专门评估

### 5.1 用户建议回顾

> "ASR-TTS 打包成一个新的包（与 agent-diva 原有后端 crate 完全独立，并主要由 GUI 引入）"

### 5.2 评估结论

**现阶段不推荐将 ASR-TTS 打包为 Rust crate。** 理由：

1. **AniPet 的 ASR 和 TTS 实现中零 Rust 代码**。ASR 使用浏览器 `SpeechRecognition` API，TTS 使用 HTTP API 调用 + `SpeechSynthesis` API。将纯前端逻辑 Rust 化是过度工程。

2. **引入 Rust crate 会增加延迟**。如果 TTS 在 Rust 端调用 API，需要：前端 → Tauri IPC → Rust → HTTP API → 返回音频 → IPC → 前端播放。而纯前端方案仅需：前端 → HTTP API → 播放。

3. **维护成本增加**。独立的 Rust crate 需要自己的 Cargo.toml、测试、CI 配置，以及与 Tauri 的集成。

### 5.3 什么情况下需要 Rust crate

**如果未来需要本地模型推理**，才需要 Rust crate：

```
未来可选：agent-diva-asr-tts/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── whisper.rs       # whisper.cpp 绑定（本地 ASR）
│   ├── piper.rs         # Piper TTS 绑定（本地 TTS）
│   └── audio.rs         # 音频采集/播放
└── build.rs             # 编译本地依赖
```

**但目前（MVP 阶段）**：
- ASR → 浏览器 Web Speech API（零成本）
- TTS → SiliconFlow CosyVoice2 API（云端，效果好）

---

## 6. 不可行/高风险项

| 项目 | 状态 | 原因 |
|------|------|------|
| Live2D Cubism 4 模型兼容 | ❌ 不可行 | Cubism 5 不向后兼容 Cubism 4 格式 |
| iOS/Android 支持 | ❌ 不可行 | 当前仅 Tauri (Windows/macOS/Linux) |
| 离线 ASR（无网络） | ⚠️ 高风险 | Web Speech API 依赖系统语音服务 |
| Live2D 模型市场/商店 | ⚠️ 高风险 | 版权和许可问题 |

---

## 7. 决策矩阵

| 决策 | 推荐方案 | 备选方案 | 理由 |
|------|----------|----------|------|
| Live2D 渲染引擎 | 原生 WebGL + Cubism5 SDK | PixiJS 6 桥接 | 减少依赖，更可控 |
| ASR Provider | Web Speech API | 云端 Whisper API | 零成本，后续可扩展 |
| TTS Provider | CosyVoice2 + 降级链 | 纯 Browser TTS | 音色质量好 |
| 前端模块组织 | `features/diva-pet/` | 独立 npm monorepo | 简化依赖管理 |
| 配置存储 | `config.json` 扩展 | 独立配置文件 | 统一管理 |
| Rust crate（现阶段） | 不需要 | — | 功能均为前端实现 |

---

## 8. Go/No-Go 建议

### ✅ Go 条件（已满足）
- [x] 技术栈兼容（Tauri 2 双发共用）
- [x] 核心代码可复用（cubism5-model, tts-service）
- [x] 增量变更，不破坏现有功能
- [x] 有明确的降级路径
- [x] 开发工时可接受（10 天内）

### ⚠️ 前置条件
- [ ] 确认 Live2D Cubism SDK 许可范围（需法务/项目负责人确认）
- [ ] 确认 `@hazart-pkg/live2d-core` 的可用性和许可
- [ ] 准备至少 1 个 Cubism 5 格式模型用于测试

### 建议
**推荐启动 MVP 开发**，优先实现 Live2D + TTS（语音输出），ASR（语音输入）作为 Phase 2 迭代。
