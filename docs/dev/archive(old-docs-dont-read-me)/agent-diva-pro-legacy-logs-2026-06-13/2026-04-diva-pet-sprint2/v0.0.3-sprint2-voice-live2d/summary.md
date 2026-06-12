# Sprint 2 — 语音 + Live2D 备选 (P1, 3天)

## 概述

Sprint 2 补齐了 Diva Pet 的语音交互能力（TTS 播报 + ASR 语音输入），同时集成了 Live2D Cubism 5 作为备选渲染器。此 Sprint 后用户拥有完整的多模态桌宠体验。

## 交付清单

### 语音能力 (2.1-2.4)

| ID | 任务 | 文件 | 行数 | 状态 |
|----|------|------|------|------|
| 2.1 | TTS 服务迁移 | `voice/services/tts-service.ts` | 1168 | ✅ |
| 2.2 | 语音播放器 | `voice/composables/useVoicePlayer.ts` | 94 | ✅ |
| 2.3 | 语音输入 | `voice/composables/useVoiceInput.ts` | 431 | ✅ |
| 2.4 | 语音面板 UI | `voice/components/DivaPetVoicePanel.vue` | ~180 | ✅ |

### Live2D 备选渲染器 (2.5-2.8)

| ID | 任务 | 文件 | 行数 | 状态 |
|----|------|------|------|------|
| 2.5 | Vendor 复制 | `public/live2d/` + `src/vendor/cubism5-framework/` | 73文件 | ✅ |
| 2.6 | Cubism 核心迁移 | `live2d/cubism5-core.ts` + `cubism5-model.ts` | 379+1523 | ✅ |
| 2.7 | Live2D 渲染组件 | `live2d/components/DivaPetAvatar.vue` | 352 | ✅ |
| 2.8 | 渲染器切换集成 | `components/DivaPetView.vue` | 268 | ✅ |

### 接口更新

| 文件 | 变更 |
|------|------|
| `index.ts` | 新增 DivaPetAvatar, DivaPetVoicePanel, useVoicePlayer, useVoiceInput, ttsService, TTSVoiceConfig 导出 |
| `types.ts` | 无变更（PetConfig 已含 renderer/ttsEnabled 字段） |

## 关键设计决策

1. **TTS 默认 Provider = Browser**：零配置，开箱即用。未来可切换到 OpenAI/SiliconFlow。
2. **ASR 方案 = Web Speech API**：浏览器原生，零成本，中文支持良好。
3. **VoiceFileReader 注入模式**：tts-service.ts 通过可注入接口加载语音参考文件，而非硬编码 Tauri IPC。
4. **Render → DivaCubism5Model.render() 单次调用**：模型内部已完成 updateModel + 清屏 + 绘制，组件不重复调用。
5. **Live2D 无 Tauri 依赖**：DivaPetAvatar 通过 `defineExpose({ loadModel })` 接受模型数据，加载逻辑由上层提供。

## 文件统计

```
features/diva-pet/
├── index.ts                                   # 已更新
├── live2d/
│   ├── cubism5-core.ts        (379行)         # 新增 - Core运行时引导
│   ├── cubism5-model.ts       (1523行)        # 新增 - 模型类
│   └── components/
│       └── DivaPetAvatar.vue  (352行)         # 新增 - Live2D渲染器
├── voice/
│   ├── composables/
│   │   ├── useVoicePlayer.ts  (94行)          # 新增 - TTS播报
│   │   └── useVoiceInput.ts   (431行)         # 新增 - 麦克风语音识别
│   ├── services/
│   │   └── tts-service.ts     (1168行)        # 新增 - TTS核心服务
│   └── components/
│       └── DivaPetVoicePanel.vue (~180行)     # 新增 - 语音面板
└── components/
    └── DivaPetView.vue        (268行)         # 已更新 - 集成所有功能

public/live2d/
├── cubism5/                   (49文件)        # 新增 - Cubism Core + Framework
└── shaders/                   (13文件)        # 新增 - WebGL Shader

src/vendor/cubism5-framework/  (47文件)        # 新增 - Vite可导入的Framework

总计: 8个新源文件, 60个vendor/shader文件, 2个修改文件
```

## 技术债/待完成

- [ ] Tauri 命令 `pet_load_live2d_bundle` — 为 Live2D 加载远程模型包
- [ ] VRM 口型同步连线 — useVrmMouthSync 已实现但未接入渲染循环
- [ ] Live2D expression-map.json — 表情关键词映射
- [ ] Live2D 自动适配视口（readPixels 方案）
- [ ] 语音提供商切换 UI（设置面板）
