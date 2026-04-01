# 测试自动化摘要

**项目：** newspace（本次针对 `agent-diva-gui`）  
**日期：** 2026-03-31  
**执行人：** Quinn 工作流（bmad-qa-generate-e2e-tests）

## 检测到的测试框架

| 层级 | 框架 | 位置 |
|------|------|------|
| 单元 / 组件 | Vitest + Vue Test Utils + jsdom | `agent-diva-gui/src/**/*.spec.ts` |
| 浏览器 E2E | Playwright | `agent-diva-gui/e2e/*.spec.ts` |

仓库根目录 `tests/` 尚不存在；E2E 落在 GUI 包内，与现有 Vite 配置和浏览器 mock 行为一致。

## 已生成的测试

### API 测试

- [ ] 未新增独立 HTTP API 规格（GUI 以 Tauri `invoke` 为主；`src/api/*.spec.ts` 等已有 Vitest 覆盖 neuro 等模块）

### E2E 测试

- [x] `agent-diva-gui/e2e/gui-browser-mode.spec.ts` — 浏览器模式（无 Tauri）下：主界面可见、发送消息得到模拟回复、生成中停止

## 覆盖率（概览）

- **API 端点：** 不适用独立 REST 套件；相关逻辑由既有 Vitest 覆盖（如 `neuro.spec.ts`）
- **UI 主路径：** 聊天输入区 `data-testid="person-main-composer"`、发送 / 停止、欢迎与 mock 回复文案已覆盖

## 运行方式

```bash
cd agent-diva/agent-diva-gui
npm run test          # Vitest
npm run test:e2e      # Playwright（会拉起 Vite :4173）
```

首次在本机执行 Playwright 时需：`npx playwright install chromium`

## 后续建议

- 在 CI 中增加 `npm run test` 与 `npm run test:e2e`（设置 `CI=1` 以启用重试与独占 webServer）
- 若需验证真实 Tauri 行为，可再增加基于 `tauri driver` 或打包后冒烟的方案（超出当前浏览器 mock 范围）

## 核对清单（workflow checklist）

- [x] E2E 已生成（UI 存在）
- [x] 使用标准 Playwright API
- [x] 覆盖主路径与关键交互（停止）
- [x] 已执行并通过：`playwright test`、`vitest run`
- [x] 使用语义化定位（`getByRole`、`getByTestId`、`getByTitle`、`getByText`）
- [x] 描述清晰；未使用固定 `sleep`（依赖 Playwright 自动等待）
- [x] 测试相互独立
- [x] 本摘要已写入本文件
