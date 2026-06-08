# Sprint 3 — 打磨 + 发布 (P2, 2天)

## 概述

Sprint 3 完成 Diva Pet 桌宠项目的打磨与发布准备工作。涵盖性能优化、错误处理完善、单元测试、国际化、模型管理 UI、冒烟测试和 CHANGELOG 更新。此 Sprint 后项目达到可发布状态。

## 交付清单

### 3.1 性能优化

| 功能 | 文件 | 描述 |
|------|------|------|
| DPR cap | `DivaVrmAvatar.vue` | `setPixelRatio(Math.min(devicePixelRatio, 2))` 限制像素比 |
| 页面不可见暂停 | `DivaVrmAvatar.vue` | Page Visibility API — 标签页隐藏时取消 rAF，恢复时重启渲染循环 |
| VRM 模型缓存 | `DivaVrmAvatar.vue` | 模块级 `Map<string, CachedVrmEntry>` 缓存已加载的 VRM 模型，再次切换时从缓存克隆场景，避免重复网络加载 |

### 3.2 错误处理完善

| 功能 | 文件 | 描述 |
|------|------|------|
| 自动重试 | `DivaVrmAvatar.vue` | 加载失败后自动重试（最多 3 次），指数退避（1s/2s/4s），错误覆盖层显示重试进度 |
| WebGL 降级 | `DivaVrmAvatar.vue` | `webglcontextlost` / `webglcontextrestored` 事件处理，上下文丢失时显示友好降级提示 |

### 3.3 单元测试（56 tests, 3 files, 0 failures）

| 测试文件 | 测试数 | 描述 |
|----------|--------|------|
| `useVrmExpression.test.ts` | 15 | 中英文关键词情绪检测、大小写不敏感、优先级规则、null 安全、响应式更新 |
| `useVrmMouthSync.test.ts` | 12 | 口型形状循环 (aa→ih→ou)、值域振荡 [0.3, 0.9]、静音/说话状态切换、null 安全 |
| `tts-service.test.ts` | 29 | 单例、VoiceFileReader 注入、浏览器 TTS、API 降级链、重试策略、stopPlayback、TTSVoiceConfig |

### 3.4 国际化 (i18n)

| 文件 | 变更 |
|------|------|
| `locales/zh.ts` | +48 行 — `pet` 顶层 section（voice / model / renderer / config 四个子模块） |
| `locales/en.ts` | +48 行 — 对应英文翻译 |

覆盖范围：语音面板、模型管理器、渲染器状态、配置项 — 全部中英文文案。

### 3.5 模型管理 UI

| 文件 | 描述 |
|------|------|
| `DivaPetModelManager.vue`（新建） | 右侧滑出面板，展示 VRM 模型列表、导入 .vrm 文件、切换当前模型、当前模型高亮标记 |
| `DivaPetView.vue`（更新） | 集成模型管理器入口（齿轮按钮）+ modelChanged 事件处理 |
| `index.ts`（更新） | 导出 DivaPetModelManager |

### 3.6 冒烟测试 + 构建验证

- **单元测试**: `npx vitest run` — 56/56 通过 (513ms)
- **类型检查**: `npx vue-tsc --noEmit` — 5 个预存错误（DivaPetVoicePanel Ref↔boolean prop 类型不匹配，Sprint 2 遗留）
- **平台**: Windows 11, Node.js v25.9.0, npm 11.12.1

### 3.7 CHANGELOG

| 文件 | 变更 |
|------|------|
| `CHANGELOG.md` | [Unreleased] 下新增 Sprint 3 条目（Added + Fixed） |

## 关键设计决策

1. **VRM 模型缓存使用 `scene.clone()` 方案**：克隆后的场景可立即渲染，VRM 实例共享缓存引用（动画更新可能受限，但渲染正常）
2. **自动重试与手动重试互斥**：手动点击"Retry"会清除自动重试 timeout 并重置计数
3. **WebGL 降级调用 `event.preventDefault()`** 以允许浏览器恢复上下文
4. **模型管理器依赖 Tauri 命令** `pet_list_vrm_models`（Sprint 1 已实现），浏览器模式下降级显示提示
5. **测试框架选型 Vitest + happy-dom**：兼容 Vite 生态，支持 Vue composable 测试

## 文件统计

```
features/diva-pet/
├── vrm/components/DivaVrmAvatar.vue        (439行, +150)  ← 性能优化+错误处理
├── vrm/composables/__tests__/
│   ├── useVrmExpression.test.ts             (243行, 新建)
│   └── useVrmMouthSync.test.ts              (238行, 新建)
├── voice/services/tts-service.ts            (1150行, -18) ← 移除死代码
├── voice/services/__tests__/
│   └── tts-service.test.ts                  (470行, 新建)
├── components/DivaPetModelManager.vue       (新建)
├── components/DivaPetView.vue               (292行, +24)  ← 集成模型管理器
└── index.ts                                 (已更新)

locales/
├── zh.ts                                    (+48行)
└── en.ts                                    (+48行)

CHANGELOG.md                                 (已更新)

总计: 4 个新文件, 5 个修改文件
```

## 技术债/待完成

- [ ] DivaPetVoicePanel 的 prop 类型修复（`Ref<boolean>` → `boolean`，5 个 vue-tsc 警告）
- [ ] Live2D 渲染器连线（`DivaPetAvatar.loadModel()` 未接入 DivaPetView）
- [ ] Tauri 命令 `pet_import_vrm_model` 实现（模型管理器导入功能的后端支撑）
- [ ] Live2D 模型加载 Tauri 命令（`pet_load_live2d_bundle`）
- [ ] 语音提供商切换 UI
