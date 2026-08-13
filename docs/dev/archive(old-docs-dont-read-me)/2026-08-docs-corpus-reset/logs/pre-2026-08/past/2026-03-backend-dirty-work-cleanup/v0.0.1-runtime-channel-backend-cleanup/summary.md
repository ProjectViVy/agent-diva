# 迭代摘要

## 目标

在不处理 GUI 优化的前提下，收口 `agent-diva` 主后端里尚未完成的脏工作，重点覆盖：

- `agent-diva-channels` 的 channel manager 一致性尾巴
- `agent-diva-manager` runtime 启动/关闭职责的内部降复杂度
- provider companion 后端路由边界的命名残留

## 完成内容

- `agent-diva-channels/src/manager.rs`
  - 抽出统一的 channel 启动 helper，避免 `start_all()` 与 `update_channel()` 各自维持不同语义。
  - 对齐 Slack 冷启动与热更新的配置准入条件，热更新现在也要求 `bot_token` 和 `app_token` 同时存在。
  - 删除“是否插入失败 handler”这类犹豫态逻辑，统一为启动失败直接返回错误，并在批量启动时汇总失败 channel。

- `agent-diva-manager/src/runtime.rs`
  - 保持 `run_local_gateway()` 对外接口不变，但将内部职责拆成子模块：
    - `runtime/bootstrap.rs`
    - `runtime/task_runtime.rs`
    - `runtime/shutdown.rs`
  - 使 bootstrap、后台任务启动、优雅关闭三类职责不再混在一个文件中。
  - 对 `bootstrap_channel_runtime()` 的语义做了更明确的记录：当前仍保持 best-effort，不因 channel 初始化失败而阻断整个 gateway。

- `agent-diva-cli/src/main.rs`
  - 抽出 `build_gateway_runtime_config(...)`，减少 CLI 直接拼装 runtime 细节。
  - 将 gateway 端口常量集中到 `agent-diva-manager::DEFAULT_GATEWAY_PORT`，避免 CLI 输出文案与 runtime 配置再次分叉。

- `agent-diva-manager/src/server.rs`
  - 将 `/api/events` 从 provider 路由组移回 runtime 路由组。
  - `provider_routes()` 现在只保留 `/api/providers*` 相关接口，使 provider companion 后端边界更干净。

## 低优先级设计债判断

- `agent-diva-providers/src/litellm.rs` 与 `agent-diva-tools/src/sanitize.rs` 目前都包含 ANSI/control-character 清洗逻辑。
- 两处实现语义基本一致，但职责层级不同：
  - `litellm.rs` 负责 provider 出站消息净化
  - `sanitize.rs` 负责工具输出与 JSON 安全化
- 本轮未继续合并，原因是它们当前不构成主后端阻塞，且强行抽共享会扩大改动面。
- 结论：保留为后续一致性优化项，而不是这轮必做实现。
