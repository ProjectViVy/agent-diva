# Sprint 3 — 发布说明

## 版本信息

- **版本号**: v0.0.4
- **Sprint**: 3 — 打磨 + 发布
- **日期**: 2026-04-29
- **发布类型**: 开发迭代（非正式发布）

## 发布内容

### 新功能

- **模型管理器 UI**: `DivaPetModelManager` 组件 — 可视化模型列表、导入 VRM 文件、切换当前模型
- **自动重试**: VRM 模型加载失败后自动重试（最多 3 次，指数退避）
- **WebGL 降级**: GPU 上下文丢失时显示友好提示
- **页面不可见暂停**: 标签页隐藏时自动停止渲染，节省 GPU 资源
- **VRM 模型缓存**: 已加载模型缓存，再次切换无需重新下载

### 测试

- **56 个单元测试**: 覆盖 `useVrmExpression` (15) / `useVrmMouthSync` (12) / `tts-service` (29)
- 测试框架: Vitest 4.1.5 + happy-dom

### 国际化

- 中英文完整覆盖 Diva Pet 全部 UI 文案（语音面板 / 模型管理器 / 渲染器状态 / 配置项）

## 变更文件

| 操作 | 文件 |
|------|------|
| 修改 | `DivaVrmAvatar.vue` (+150 行: 性能优化 + 错误处理) |
| 修改 | `DivaPetView.vue` (+24 行: 模型管理器集成) |
| 修改 | `index.ts` (+1 导出) |
| 修改 | `zh.ts` (+48 行: pet i18n) |
| 修改 | `en.ts` (+48 行: pet i18n) |
| 修改 | `CHANGELOG.md` (Sprint 3 条目) |
| 新建 | `DivaPetModelManager.vue` |
| 新建 | `useVrmExpression.test.ts` (243 行) |
| 新建 | `useVrmMouthSync.test.ts` (238 行) |
| 新建 | `tts-service.test.ts` (470 行) |
| 新建 | `vitest.config.ts` |
| 新建 | `package.json` (test 脚本 + vitest devDeps) |

## 向后兼容

- 所有变更均为增量式，不破坏现有 Chat / Settings / Cron 功能
- API 接口无变化（`index.ts` 仅新增导出）
- 配置格式无变化（`PetConfig` 类型未修改）

## 已知问题

| 问题 | 严重性 | 计划 |
|------|--------|------|
| Live2D 渲染器未接入 | 🟡 中等 | 下个迭代 |
| `pet_import_vrm_model` Tauri 命令缺失 | 🟡 中等 | 下个迭代 |
| DivaPetVoicePanel props 类型 (5 个 vue-tsc 警告) | 🟢 低 | 下个迭代 |
