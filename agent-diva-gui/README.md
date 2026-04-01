# Agent Diva GUI

Agent Diva 的图形化桌面客户端，基于 Tauri + Vue 3 构建。

## 功能特性

*   **实时对话**: 与 Agent Diva 进行自然语言交互。
*   **流式响应**: 像 ChatGPT 一样实时显示 Agent 的思考和回复过程。
*   **工具可视化**: 显示 Agent 调用的工具及其结果。
*   **动态配置**: 在应用内直接配置 API Key、模型和 API 地址。
*   **外部 Hook**: 提供 HTTP 接口，允许外部脚本或工具向 Agent 发送消息。

## 开发与运行

### 前置要求

*   Node.js (推荐 v18+)
*   Rust (最新稳定版)
*   pnpm (推荐) 或 npm

### 启动开发环境

1.  进入 GUI 目录:
    ```bash
    cd agent-diva-gui
    ```

2.  安装依赖:
    ```bash
    pnpm install
    ```

3.  启动开发模式:
    ```bash
    pnpm tauri dev
    ```

## 外部 Hook 使用

应用启动后，会在后台监听 `3000` 端口。你可以通过 HTTP POST 请求向 GUI 发送消息：

```bash
curl -X POST http://localhost:3000/api/hook/message \
  -H "Content-Type: application/json" \
  -d '{"content": "Hello from external tool!"}'
```

消息将立即出现在聊天界面中。

## 配置说明

首次运行时，如果未设置环境变量 `LITELLM_API_KEY` 等，发送消息会提示配置。
点击右上角的设置图标（⚙️）即可配置：

*   **API Base URL**: LLM 服务地址 (例如 `https://api.openai.com/v1` 或本地 `http://localhost:4000`)
*   **API Key**: 你的 API 密钥
*   **Model**: 模型名称 (例如 `gpt-3.5-turbo`, `anthropic/claude-3-opus`)

## 附录：蜂群「关大脑皮层」语义（FR17）

GUI 贡献者若需了解 **大脑皮层开/关** 的产品语义、行走路径及与无头测试的追溯关系，请参阅蜂群仓库中的维护者文档：

*   [`agent-diva-swarm/docs/CORTEX_OFF_FR17_MAINTAINER_GUIDE.md`](../agent-diva-swarm/docs/CORTEX_OFF_FR17_MAINTAINER_GUIDE.md)（FR17 主文档）
*   技术真值与测试对照表：[`CORTEX_OFF_SIMPLIFIED_MODE.md`](../agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md)
*   FR21 冻结选型：[`ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`](../agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md)

## 附录：运行遥测挂点冻结（FR22 / Story 2.4）

**对外暴露面（三选一，已冻结）：** 桌面 GUI **设置 → 高级**，通过 Tauri `get_run_telemetry_snapshot` **invoke** 读取最近一次聊天 SSE 中的 `run_telemetry` 事件缓存；**不**使用聊天主区、**不**默认写入用户 transcript（NFR-R2）。

**线协议：** `RunTelemetrySnapshotV0`（`schemaVersion`、`internalStepCount`、`preludeLlmCalls`、`phaseCount`、可选 `fullSwarmConvergenceRounds`、可选 `overSuggestedBudget`），与 `agent_diva_core::bus::RunTelemetrySnapshotV0` 及网关 SSE 事件名 `run_telemetry` 一致。字段语义维护者单点见 `agent-diva-swarm/docs/RUN_TELEMETRY_FR22.md`（Story 5.2）。

**可选 feature-flag：** 本地存储键 `agentDivaFeatureRunTelemetry`（`1` = 开启开发者遥测 UI）。
