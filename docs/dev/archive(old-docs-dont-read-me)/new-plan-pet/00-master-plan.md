# Diva Pet — 敏捷开发总体规划

> Agent Diva 桌宠增强项目（VRM + Live2D + ASR + TTS）
>
> 统筹 `diva-pet-integration/` (Live2D 方案) 和 `vrm-integration/` (VRM 方案)

---

## 1. 项目愿景

为 Agent Diva GUI 添加一个**可选**的虚拟角色交互层。用户可以看到一个 3D/2D 角色，角色能根据对话内容做出表情反应，能语音播报 AI 回复，能接收语音输入。

**核心原则：**
- **不破坏现有功能** — 所有变更为增量式
- **可选开关** — 用户可随时启用/禁用
- **Session 共享** — 与 Chat 页面共用同一对话 session
- **对后端零影响** — Diva 网关完全无感知

---

## 2. 优先级定义

| 优先级 | 含义 | 决策标准 |
|--------|------|----------|
| **P0** | 必须交付（MVP） | 缺了它就不是"桌宠"了 |
| **P1** | 应该交付 | 显著提升体验，但可延期 |
| **P2** | 可以交付 | 锦上添花，资源允许就做 |
| **P3** | 未来考虑 | 下个版本再议 |

---

## 3. 迭代周期总览

```
Sprint 0 ──── Sprint 1 ──── Sprint 2 ──── Sprint 3
(3 天)        (4 天)        (3 天)        (2 天)
P0 基础设施    P0 VRM MVP    P1 语音+Live2D P2 打磨+发布
```

| Sprint | 主题 | 优先级 | 工期 | 累计交付 |
|--------|------|--------|------|----------|
| **Sprint 0** | 基础设施 + Session 集成 | P0 | 3d | 可对话的空壳 |
| **Sprint 1** | VRM 3D 角色 MVP | P0 | 4d | 3D 角色可看可聊 |
| **Sprint 2** | 语音 + Live2D 备选 | P1 | 3d | 语音交互 + 双渲染器 |
| **Sprint 3** | 打磨 + 发布 | P2 | 2d | 可发布版本 |

---

## 4. Sprint 0 — 基础设施 + Session 集成（P0, 3 天）

> **目标**：搭建桌宠模块骨架，打通 Chat Session 共享通道。此阶段还没有角色渲染，但消息已经可以在 DivaPet 页面中流通。

### 4.1 任务清单

| ID | 任务 | 优先级 | 工时 | 验收 |
|----|------|--------|------|------|
| 0.1 | 创建 `features/diva-pet/` 目录骨架 | P0 | 0.5d | 目录存在，index.ts 正常导出 |
| 0.2 | 安装 VRM npm 依赖 (`three`, `@pixiv/three-vrm`) | P0 | 0.5d | `pnpm install` 成功 |
| 0.3 | 修改 `NormalMode.vue`，添加 "Diva Pet" 侧边栏入口 | P0 | 0.5d | 点击切换到 DivaPetView |
| 0.4 | 实现 `DivaPetView.vue` 壳（接收 messages/isTyping props, emit send） | P0 | 1d | 能看到 messages 列表（纯文字），能发送消息到 Chat session |
| 0.5 | 修改 `config.json` 添加 `pet` section（含 enabled/disabled 开关） | P0 | 0.5d | 通过配置可控制侧边栏入口显隐 |

### 4.2 交付物

```
features/diva-pet/
├── index.ts
├── types.ts
├── components/
│   └── DivaPetView.vue          ← 壳（仅文字，无角色）
└── services/
    └── pet-config.ts            ← 配置读取

修改:
├── NormalMode.vue               ← 添加侧边栏入口
├── config.json (模板)            ← 新增 pet section
└── package.json                  ← 新增 VRM 依赖
```

### 4.3 验收标准

- [ ] 侧边栏出现 "Diva Pet" 按钮
- [ ] 点击切换到 DivaPet 页面（目前显示 messages 文字列表）
- [ ] 在 DivaPet 页面输入文字 → ChatView 同步显示
- [ ] 在 ChatView 输入文字 → DivaPet 页面同步显示
- [ ] `pet.enabled = false` → 侧边栏入口消失
- [ ] `pet.enabled = true` → 侧边栏入口恢复

---

## 5. Sprint 1 — VRM 3D 角色 MVP（P0, 4 天）

> **目标**：实现基于 `@pixiv/three-vrm` 的 3D 角色渲染，角色能对对话做出表情反应。这是核心交付——用户第一次"看到"桌宠。

### 5.1 任务清单

| ID | 任务 | 优先级 | 工时 | 依赖 |
|----|------|--------|------|------|
| 1.1 | 实现 `DivaVrmAvatar.vue` — Three.js 初始化 + VRM 加载 + 渲染循环 | P0 | 1.5d | 0.4 |
| 1.2 | 准备默认 VRM 模型（CC0 许可）放到 `public/vrm/models/` | P0 | 0.5d | — |
| 1.3 | 实现 `useVrmExpression.ts` — 情绪关键词推断 + expressionManager 驱动 | P0 | 1d | 1.1 |
| 1.4 | Tauri command: `pet_list_vrm_models` — 扫描 vrm 目录返回模型列表 | P0 | 0.5d | — |
| 1.5 | 实现 `useVrmMouthSync.ts` — 简化版正弦波口型同步 | P1 | 0.5d | 1.1 |

### 5.2 交付物

```
features/diva-pet/
├── vrm/
│   ├── components/
│   │   └── DivaVrmAvatar.vue        ← 3D 角色渲染
│   └── composables/
│       ├── useVrmExpression.ts      ← 表情推断
│       └── useVrmMouthSync.ts       ← 口型同步
├── components/
│   └── DivaPetView.vue              ← 更新：集成 VRM + 气泡
│
public/vrm/
├── models/
│   └── alice.vrm                    ← 默认模型 (CC0)
└── animations/                      ← 动画文件 (可选)
```

### 5.3 验收标准

- [ ] 进入 Diva Pet 页面，3D VRM 角色在 5s 内渲染完成
- [ ] 鼠标拖拽旋转视角，滚轮缩放
- [ ] Agent 回复含 "哈哈" → 角色表情 happy
- [ ] Agent 回复含 "难过" → 角色表情 sad
- [ ] TTS 播放时嘴巴开合动画
- [ ] 消息气泡同步显示最新 Agent 回复
- [ ] 加载不存在的模型 → 错误提示
- [ ] WebGL 不可用 → 降级提示

---

## 6. Sprint 2 — 语音 + Live2D 备选（P1, 3 天）

> **目标**：补齐语音交互能力（TTS 播报 + ASR 语音输入），同时集成 Live2D 作为备选渲染器。此 Sprint 后用户有完整的多模态体验。

### 6.1 任务清单

| ID | 任务 | 优先级 | 工时 | 依赖 |
|----|------|--------|------|------|
| **语音能力** | | | | |
| 2.1 | 迁移 `tts-service.ts`（从 AniPet，框架无关纯 TS 类） | P1 | 0.5d | — |
| 2.2 | 实现 `useVoicePlayer.ts` — 监听 agent-response-complete → TTS 播放 | P1 | 0.5d | 2.1, 1.1 |
| 2.3 | 实现 `useVoiceInput.ts` — Web Speech API 语音输入 → sendMessage | P1 | 0.5d | 0.4 |
| 2.4 | 实现 `DivaPetVoicePanel.vue` — 语音控制面板 UI | P1 | 0.5d | 2.2, 2.3 |
| **Live2D 备选** | | | | |
| 2.5 | 安装 Live2D npm 依赖 + 复制 vendor 脚本到 `public/live2d/` | P1 | 0.5d | — |
| 2.6 | 迁移 `cubism5-core.ts` + `cubism5-model.ts`（从 AniPet） | P1 | 0.5d | 2.5 |
| 2.7 | 实现 `DivaPetAvatar.vue`（Live2D 版，React → Vue 3 重写） | P1 | 1d | 2.6 |
| 2.8 | `DivaPetView.vue` 添加渲染器切换逻辑 (`v-if="renderer"`) | P1 | 0.5d | 1.1, 2.7 |

### 6.2 交付物

```
features/diva-pet/
├── live2d/
│   ├── cubism5-core.ts
│   ├── cubism5-model.ts
│   └── components/
│       └── DivaPetAvatar.vue        ← Live2D 2D 渲染
├── voice/
│   ├── composables/
│   │   ├── useVoicePlayer.ts
│   │   └── useVoiceInput.ts
│   ├── services/
│   │   └── tts-service.ts
│   └── components/
│       └── DivaPetVoicePanel.vue
└── components/
    └── DivaPetView.vue              ← 更新：集成 VRM + Live2D + Voice

public/
├── live2d/cubism5/                   ← Live2D vendor 脚本
└── vrm/                              ← VRM 资源
```

### 6.3 验收标准

- [ ] Agent 回复自动触发 TTS 语音播报
- [ ] 无 API Key 时降级使用浏览器 TTS
- [ ] 点击麦克风按钮 → 语音识别 → 消息发送到 Chat session
- [ ] 切换渲染器为 Live2D → Live2D 角色正常渲染并响应对话
- [ ] VRM 和 Live2D 切换时旧渲染器正确释放
- [ ] 语音控制面板显示正确的播放/监听状态

---

## 7. Sprint 3 — 打磨 + 发布（P2, 2 天）

> **目标**：性能优化、边界处理、测试、文档完善，达到可发布状态。

### 7.1 任务清单

| ID | 任务 | 优先级 | 工时 |
|----|------|--------|------|
| 3.1 | 性能优化：DPR cap、页面不可见暂停渲染、模型缓存 | P2 | 0.5d |
| 3.2 | 错误处理完善：加载失败重试、模型损坏提示、WebGL 降级 | P2 | 0.5d |
| 3.3 | 单元测试：useVrmExpression, useVrmMouthSync, tts-service | P2 | 0.5d |
| 3.4 | 国际化：中英文所有新增文案 | P2 | 0.5d |
| 3.5 | 模型管理 UI：DivaPetModelManager（导入/切换/预览） | P2 | 0.5d |
| 3.6 | 冒烟测试 + 构建验证 (Windows) | P2 | 0.5d |
| 3.7 | CHANGELOG + Release Notes | P2 | 0.5d |

### 7.2 验收标准

- [ ] Windows 构建通过
- [ ] 30 分钟运行无崩溃
- [ ] 全部 P0 回归测试通过（Chat/Settings/Cron 不受影响）
- [ ] 文档完善（README 含 Diva Pet 使用指南）

---

## 8. P3 — 未来迭代（不在本次范围）

| ID | 功能 | 说明 |
|----|------|------|
| F1 | 3D 场景（Gaussian Splatting） | 角色身后的 3D 环境 |
| F2 | 高级口型同步（Audio Analyser FFT） | 替代正弦波，更自然的唇形 |
| F3 | VRM 动画系统（.vrma） | 打招呼、舞蹈等预设动作 |
| F4 | 独立窗口模式 | Diva Pet 作为独立 Tauri 窗口 |
| F5 | WebXR 全景支持 | VR/AR 头显 |
| F6 | 多角色支持 | 同时渲染多个角色 |

---

## 9. 风险管理

| 风险 | Sprint | 等级 | 缓解措施 |
|------|--------|------|----------|
| Live2D SDK 许可不兼容 MIT | S2 | 🔴 | **VRM 优先 (S1)**，Live2D 降级为 P3 或移除 |
| @pixiv/three-vrm 与 Three.js 版本冲突 | S1 | 🟡 | 精确版本锁定，Sprint 0 提前验证 |
| WebGL 在低端设备性能不足 | S1 | 🟡 | DPR cap + 降级到静态图 |
| VRM 模型许可不明确 | S1 | 🟡 | S1 阶段使用确认 CC0 的模型 |
| 从 AniPet 复制的代码有潜在类型问题 | S2 | 🟢 | tts-service 是框架无关的纯 TS，风险低 |

---

## 10. 人员与资源

| 角色 | 投入 | Sprint 0-3 总工时 |
|------|------|-------------------|
| 前端开发 (Vue 3 + Three.js) | 全职 | 12 天 |
| Rust 开发 (Tauri commands) | 兼职 | 1 天 |
| QA | 兼职 (Sprint 3) | 0.5 天 |

> 如为单人开发，总工期 12 个工作日（约 2.5 周）。多人协作可压缩至 1.5 周。

---

## 11. 关键决策记录

| 决策 | 选项 | 选择 | 理由 |
|------|------|------|------|
| 首选渲染引擎 | VRM / Live2D | **VRM** | MIT 许可，无合规风险 |
| Live2D 定位 | 必需 / 备选 | **备选 (P1)** | 低端设备 + 已有 Live2D 资产用户 |
| TTS 默认 Provider | SiliconFlow / Browser | **Browser** | 零配置，开箱即用 |
| ASR 方案 | Web Speech / 云端 | **Web Speech** | 零成本，浏览器原生 |
| 配置开关 | 独立配置文件 / config.json | **config.json pet section** | 统一管理 |
| 独立窗口 | Sprint 3 / P3 | **P3** | MVP 先嵌入主窗口 |

---

## 12. 参考项目索引

以下为根目录下的两个参考实现，本文档中所有"从 AniPet 迁移"、"参照 super-agent-party" 等描述均指以下具体文件：

### 12.1 AniPet（Live2D + TTS/ASR 参考）

| 参考内容 | 精确路径 | 行数 |
|----------|----------|------|
| Cubism 5 运行时管理 | `AniPet/apps/desktop/src/components/live2d-avatar/cubism5-core.ts` | 379 行 |
| Live2D 模型加载/渲染 | `AniPet/apps/desktop/src/components/live2d-avatar/cubism5-model.ts` | 1524 行 |
| React 渲染组件 | `AniPet/apps/desktop/src/components/live2d-avatar/Live2DAvatarRenderer.tsx` | 1373 行 |
| TTS 核心服务 | `AniPet/apps/desktop/src/features/voice/tts-service.ts` | 1048 行 |
| ASR 语音输入 Hook | `AniPet/apps/desktop/src/features/voice/use-voice-input.ts` | 402 行 |
| 语音播放 Hook | `AniPet/apps/desktop/src/features/voice/use-voice-player.ts` | 204 行 |
| 角色渲染外壳 | `AniPet/apps/desktop/src/components/avatar-renderer/AvatarRenderer.tsx` | 187 行 |
| 桌宠主壳 | `AniPet/apps/desktop/src/components/pet-shell/PetShell.tsx` | 474 行 |
| Cubism 5 Framework (vendor) | `AniPet/apps/desktop/src/vendor/cubism5-framework/` | 目录 |
| Cubism 5 Core Wasm | `AniPet/vendor/official-live2dcubismcore.min.js` | 二进制 |
| 默认 Live2D 模型 | `AniPet/live2d_resource/default/mao_pro.model3.json` | JSON |
| 表情定义 | `AniPet/live2d_resource/default/expressions/exp_01.exp3.json` ~ `exp_08.exp3.json` | 8 个 |
| 动作数据 | `AniPet/live2d_resource/default/motions/mtn_01.motion3.json` ~ `mtn_04.motion3.json` | 4 个 |
| 物理配置 | `AniPet/live2d_resource/default/mao_pro.physics3.json` | JSON |
| 姿态配置 | `AniPet/live2d_resource/default/mao_pro.pose3.json` | JSON |

### 12.2 super-agent-party（VRM 3D 参考）

| 参考内容 | 精确路径 | 行数/大小 |
|----------|----------|-----------|
| VRM 入口 HTML | `super-agent-party/static/vrm.html` | 76 行 |
| VRM 核心 JS | `super-agent-party/static/js/vrm.js` | 4573 行 (188KB) |
| 默认 VRM 模型 | `super-agent-party/vrm/Alice.vrm` | 二进制 |
| 备选 VRM 模型 | `super-agent-party/vrm/Bob.vrm` | 二进制 |
| VRM 动画 (11个) | `super-agent-party/vrm/animations/*.vrma` | 11 文件 |
| 3D 场景文件 | `super-agent-party/vrm/scene/*.spz` | 3 文件 |
| Three.js 库 | `super-agent-party/static/libs/three/` | 目录 |
| @pixiv/three-vrm 库 | `super-agent-party/static/libs/@pixiv/three-vrm/` | 目录 |

> **许可警告**：super-agent-party 整体为 AGPL-3.0 许可，`vrm.js` 不可直接复制源码。仅可参考其架构思路和 `@pixiv/three-vrm`（MIT）的 API 用法。

### 12.3 vrm.js 关键函数索引

| 功能 | 函数名 | 行号 |
|------|--------|------|
| VRM 模型切换 | `switchToModel()` | ~L4313 |
| VRM 动画加载 | `loadVRMAAnimation()` | ~L1000 |
| 空闲动画循环 | `IdleAnimationManager` class | ~L536 |
| 程序化呼吸动画 | `createBreathClip()` | ~L1254 |
| 眨眼动画 | `createBlinkClip()` | ~L1282 |
| 口型同步（音频分析） | `startLipSyncForChunk()` | ~L1537 |
| 口型逐帧动画 | `animateChunk()` | ~L1423 |
| 表情应用逻辑 | chunk animation expression | ~L1494-1524 |
| 表情+口型混合更新 | render chunk update | ~L1656-1677 |
| 模型加载后默认表情 | VRM load callback | ~L1780 |
| 获取可用表情列表 | `getVMCBlendData()` | ~L2064 |
| 高斯场景加载 | `loadGaussScene()` | ~L342 |
| 自然姿态设置 | `setNaturalPose()` | ~L449 |

---

## 13. 附录：Sprint 看板（建议）

```
BACKLOG          SPRINT 0 (3d)    SPRINT 1 (4d)    SPRINT 2 (3d)    SPRINT 3 (2d)
────────         ─────────────    ─────────────    ─────────────    ─────────────
Live2D渲染器      ██ 0.1 目录     ██ 1.1 Avatar    ██ 2.1 TTS      ██ 3.1 性能
VRM动画系统       ██ 0.2 依赖     ██ 1.2 模型      ██ 2.2 Player   ██ 3.2 错误
3D场景            ██ 0.3 侧边栏   ██ 1.3 表情      ██ 2.3 Input    ██ 3.3 测试
独立窗口          ██ 0.4 View壳   ██ 1.4 Rust命令  ██ 2.4 Panel    ██ 3.4 i18n
多角色            ██ 0.5 config    █ 1.5 口型      ██ 2.5 Live2D   ██ 3.5 模型UI
WebXR             ░░░░░░░░░░░░    ░░░░░░░░░░░░     █ 2.6 cubism   ██ 3.6 冒烟
高级口型          ░░░░░░░░░░░░    ░░░░░░░░░░░░     ██ 2.7 Avatar2  █ 3.7 CHANGELOG
                                               █ 2.8 切换
                                              ░░░░░░░░░░░░
```

```
██ = 当前 Sprint
░░ = 已完成
   = Backlog
```
