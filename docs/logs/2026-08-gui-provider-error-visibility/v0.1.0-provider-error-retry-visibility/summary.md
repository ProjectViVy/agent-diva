# Summary — GUI Provider 错误/重试可见性修复

- 版本：`v0.1.0-provider-error-retry-visibility`
- 日期：2026-08-06
- 类型：BUG 修复 + 状态可见性增强（ERROR-SILENT + RETRY-VISIBILITY）

## 背景

用户观测（2026-08-06）：DeepSeek 连接失败（`unexpected EOF during handshake`）
重试 4 次约 128s 后失败，GUI 全程无任何提示。已记录两条 sev-P2 TODO：
`GUI-PROVIDER-ERROR-SILENT`（最终失败无错误气泡）、`GUI-PROVIDER-RETRY-VISIBILITY`
（重试期间无「未响应，重试 (n/m)」状态）。本次按已批准计划修复。

## 根因（勘察结论）

1. **主根因（ERROR-SILENT）**：`manager/runtime_control.rs` `handle_chat` 转发循环
   用 60s 无事件超时静默断开 SSE——provider 重试 128s 期间无任何 bus 事件，60s 后
   转发任务退出，128s 后 agent 发出的 `AgentEvent::Error` 无人转发，错误丢失。
2. **次根因**：Tauri 桥 SSE 循环意外结束时静默 `Ok(())`（无 final/error 事件），
   前端 `isTyping` 永久 true、占位无限转圈。
3. **RETRY-VISIBILITY**：`send_with_retry` 只有日志，无事件通道；`AgentEvent`
   无 Retry 变体；SSE/Tauri/前端无重试概念。

## 做了什么（7 个 commit）

1. **`cf567087` feat(core)**：`AgentEvent::ProviderRetry { model, attempt,
   max_retries, delay_ms, reason }` + `AgentEvent::ProviderStalled { model }`；
   serde roundtrip 测试。
2. **`78b813c0` feat(providers)**：`RetryAttempt`/`RetryListener`（Arc 闭包）类型；
   `send_with_retry` 加 `on_retry` 回调参数；`LLMProvider` 默认方法
   `set_retry_listener`；OpenAiCompatible/Anthropic 存储并 snapshot listener
   （设置与调用间无 await → 并发安全）；`ProviderTap` 转发（关键——否则 listener
   到不了 inner client）。retry 回调单测（本地 TCP mock，no_proxy 防系统代理 502）。
3. **`51c6c949` feat(agent)**：`start_model_stream` 每次 `chat_stream` 调用前注入
   listener（bus publish `ProviderRetry`，捕获 channel/chat_id），调用后清除；
   mock 集成测试断言 bus 收到重试事件。
4. **`97a2195e` fix(manager)**：`handle_chat` 转发循环重写为
   `forward_chat_events`——60s 无事件发一次 `ProviderStalled`（非终止，流保持），
   再 60s 无事件发明确断开错误并终止；`provider_retry`/`provider_stalled` 映射为
   新 SSE 事件。3 个测试（start_paused 时间推进验证 stalled/断开/过滤）。
5. **`5ad64b5e` fix(gui)**：Tauri 桥转发 `provider_retry`/`provider_stalled`（新
   payload 结构）；`saw_terminal` 跟踪——SSE 流无 final/error 结束时 emit
   `agent-error`（连接已断开），前端不再永久挂起；background 流显式忽略状态事件。
6. **`d4050d38` feat(gui)**：`Message` 加 `retryStatus`/`stalled`；App.vue 监听
   两个新事件更新当前流式消息；ChatView 渲染「未响应，重试 (n/m)」琥珀徽标与
   「仍在等待 Provider 响应」提示（仅 `isStreaming` 时显示，完成即消失）；
   i18n zh/en。3 个 vitest 渲染测试。
7. **`87e4c972` fix(e2e)**：e2e 事件收集器穷尽匹配补新变体（clippy E0004）。

## 影响范围

- `agent-diva-core`：AgentEvent 新变体。
- `agent-diva-providers`：retry 回调通道 + trait 默认方法 + 3 个实现。
- `agent-diva-agent`：listener 注入。
- `agent-diva-manager`：SSE 映射 + 转发循环（不再静默丢错误）。
- `agent-diva-gui`：src-tauri 桥 + App.vue/ChatView/i18n。
- `agent-diva-e2e`：事件收集器穷尽匹配。

## 未做（下一迭代）

- L2854/L3140 两个 plan 流式 Tauri command 的断流兜底（`saw_terminal` 未覆盖，
  已记 TODOLIST）。
- background 流（cron）重试状态前端展示（无 request_id 通道，暂忽略）。
- 预存在：`examples/minimax_sync_tts.rs` clippy `useless_conversion`（Rust 1.94
  新 lint，`just check`/`just ci` 不受影响，已记 TODOLIST）。
