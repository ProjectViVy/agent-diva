# Agent Diva GUI 产品上下文

## 产品定位

Agent Diva 是模块化 AI 助手框架，GUI 是其桌面控制面应用，用于管理本地/远程 Agent Diva 网关，提供与 AI 助手的对话界面。

## 核心用户场景

1. **对话交互**：用户与 DIVA 进行流式对话，支持 Markdown、思考过程、工具调用展示
2. **会话管理**：多会话历史、新建/切换/加载会话
3. **配置管理**：LLM 提供商（API Base、Key、Model）、工具配置（搜索、抓取）
4. **网关管理**：控制台、Cron 任务、服务启停（Windows Service / systemd / launchd）
5. **设置**：通用、提供商、通道、网络、语言、关于

## 功能模块

| 模块 | 入口 | 说明 |
|------|------|------|
| Chat | 主界面 | 对话、模型切换、历史下拉、停止生成 |
| Settings | 设置面板 | 通用/提供商/通道/网络/语言/关于 |
| Console | 控制台 | 网关日志、状态 |
| Cron | 定时任务 | Cron 任务管理 |
| Neuro | 预留 | 未来扩展 |

## 用户角色

- **桌面用户**：安装 GUI 安装包，通过图形界面与 DIVA 对话、配置
- **高级用户**：同时使用 GUI 与 Headless 模式，通过 Manager API 管理

## 技术边界

- 前端：Tauri 2 + Vue 3 + Vite + Tailwind
- 后端：Rust workspace（agent-diva-cli、agent-diva-service）
- 数据：localStorage（模型、偏好）、Tauri commands 与 gateway 通信

## 相关文档

- `docs/app-building/README.md`：构建与 WBS 索引
- `docs/app-building/优先级.md`：CA 清单与阶段
- `docs/app-building/wbs-gui-cross-platform-app.md`：GUI 架构与打包
