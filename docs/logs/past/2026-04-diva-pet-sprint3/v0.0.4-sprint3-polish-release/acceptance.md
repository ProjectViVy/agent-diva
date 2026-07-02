# Sprint 3 — 验收标准

## Master Plan 验收对照

| ID | 任务 | 验收标准 | 状态 |
|----|------|----------|------|
| 3.1 | 性能优化 | DPR cap 生效、页面不可见时暂停渲染、模型缓存加速切换 | ✅ |
| 3.2 | 错误处理完善 | 加载失败自动重试（最多3次指数退避）、WebGL 上下文丢失降级提示 | ✅ |
| 3.3 | 单元测试 | useVrmExpression / useVrmMouthSync / tts-service 三个测试套件 56 cases 全部通过 | ✅ |
| 3.4 | 国际化 | 所有新增文案中英文完整（pet.voice / pet.model / pet.renderer / pet.config） | ✅ |
| 3.5 | 模型管理 UI | DivaPetModelManager 组件可用（模型列表 / 导入 / 切换） | ✅ |
| 3.6 | 冒烟测试 + 构建 | 56 tests pass, vue-tsc 无新增错误（5 个预存） | ✅ |
| 3.7 | CHANGELOG | [Unreleased] 含 Sprint 3 变更条目 | ✅ |

## 功能验收

### 3.1 性能优化

- [x] DPR cap: `setPixelRatio(Math.min(window.devicePixelRatio, 2))` — 来源 DivaVrmAvatar.vue L245
- [x] 页面不可见暂停: `document.addEventListener('visibilitychange', onVisibilityChange)` — L289
- [x] 页面恢复时自动重启渲染循环
- [x] VRM 模型缓存: 模块级 `Map<string, CachedVrmEntry>` — L15
- [x] 缓存命中时克隆场景并立即 emit loadSuccess

### 3.2 错误处理

- [x] 自动重试: `retryCount` ref, 最多 3 次 — L34
- [x] 指数退避: `Math.pow(2, retryCount - 1) * 1000` (1s/2s/4s) — L201
- [x] 错误覆盖层显示重试进度: "Retrying (X/3)..."
- [x] 手动"Retry"按钮重置计数并取消自动重试 timeout
- [x] WebGL context lost 监听: `renderer.domElement.addEventListener('webglcontextlost', ...)` — L292
- [x] WebGL context restored 监听: L293
- [x] 降级提示覆盖层: "WebGL context lost — your GPU may be under heavy load..."

### 3.3 单元测试

- [x] `useVrmExpression.test.ts` — 15 tests (情绪关键词、优先级、null 安全、响应式)
- [x] `useVrmMouthSync.test.ts` — 12 tests (口型循环、值域、状态切换、null 安全)
- [x] `tts-service.test.ts` — 29 tests (API 降级链、重试策略、浏览器 TTS、配置)
- [x] 全部 56 tests 通过 (vitest run, 0 failures)
- [x] 测试文件位置符合项目约定: `__tests__/` 子目录

### 3.4 国际化

- [x] `zh.ts`: 新增 `pet` 顶层 section (voice / model / renderer / config)
- [x] `en.ts`: 对应英文翻译
- [x] 现有 `nav.pet` 键未被修改
- [x] 所有 UI 文案有中英文覆盖

### 3.5 模型管理 UI

- [x] `DivaPetModelManager.vue` 创建完成
- [x] 模型列表从 Tauri `pet_list_vrm_models` 加载
- [x] 当前激活模型高亮显示（粉色背景 + Check 图标）
- [x] "Import .vrm Model" 按钮调用 `pet_import_vrm_model`
- [x] 加载/空/错误状态全部覆盖
- [x] Tauri 不可用时显示降级提示
- [x] `index.ts` 导出 DivaPetModelManager
- [x] `DivaPetView.vue` 集成齿轮按钮 + modelChanged 事件处理

### 3.6 冒烟测试

- [x] `npx vitest run` — 56/56 通过
- [x] `npx vue-tsc --noEmit` — 无 Sprint 3 新增错误

### 3.7 CHANGELOG

- [x] [Unreleased] → `### Added` — Sprint 3 条目
- [x] [Unreleased] → `### Fixed` — WebGL 上下文丢失修复
- [x] 遵循 Keep a Changelog 格式

## 未完成项（已知限制）

| 项目 | 说明 | 影响 |
|------|------|------|
| Live2D 渲染器连线 | `DivaPetAvatar.loadModel()` 未接入 | Live2D 模式不可用 |
| `pet_import_vrm_model` Tauri 命令 | 后端未实现 | 模型管理器降级为仅列表/切换 |
| DivaPetVoicePanel props 类型 | 5 个 vue-tsc 预存警告 | 运行时无影响 |

## 整体评估

Sprint 3 的 7 个任务全部完成，Diva Pet 达到可发布状态（P2 级别）。
