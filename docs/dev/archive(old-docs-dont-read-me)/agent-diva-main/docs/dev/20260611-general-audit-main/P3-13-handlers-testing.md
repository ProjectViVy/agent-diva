# P3-13: handlers.rs 零测试

## 问题描述

`agent-diva-manager/src/handlers.rs` 约 880 行，包含大量 HTTP handler，但文件内没有 `#[cfg(test)]`、`#[test]` 或 `#[tokio::test]`。

该文件覆盖的关键入口包括：

- `chat_handler`：SSE chat 入口，包含 `/stop` 特殊路径和 `ManagerCommand::Chat` 发送。
- `stop_chat_handler`、`reset_session_handler`、`get_sessions_handler`、`get_session_history_handler`、`delete_session_handler`：通过 oneshot 与 manager runtime 通信。
- `events_handler`：订阅 bus event 并转成 SSE event。
- `get_config_handler`、`update_config_handler`、`get_channels_handler`、`get_tools_handler`、`update_tools_handler`、`update_channel_handler`：配置和工具控制面。
- MCP、skill upload、file upload、cron handlers：涉及 multipart、路径参数和错误响应。

其中多个函数有分支和兼容逻辑。例如 `get_session_history_handler` 和 `do_delete_session` 会在 path id 不包含 `:` 时自动补 `gui:`：

```rust
let session_key = if !id.contains(':') {
    format!("gui:{}", id)
} else {
    id
};
```

这些行为目前没有单元测试或 handler 级集成测试保护。

## 影响评估

- 回归风险：HTTP 响应 JSON shape、SSE event name、session key 兼容逻辑变更后难以及时发现。
- 稳定性风险：oneshot 接收失败、manager command 发送失败等错误路径依赖手工验证。
- 安全与数据风险：file/skill upload 的缺字段、大小限制、文件名处理等边界缺少测试容易引入漏洞。
- 维护成本：handlers.rs 是控制面核心入口，零测试会让后续拆分和重构成本变高。

## 解决方案

建立 handler 测试分层：

1. 纯分支提取为可测试函数。
2. 对 command/oneshot 路径使用 fake `AppState` 或测试 runtime。
3. 对 HTTP 层使用 `tower::ServiceExt` 构造 axum Router 做集成测试。

建议先提取 session key 兼容逻辑：

```rust
fn normalize_session_key(id: String) -> String {
    if id.contains(':') {
        id
    } else {
        format!("gui:{id}")
    }
}
```

然后补测试：

```rust
#[test]
fn normalize_session_key_preserves_explicit_channel() {
    assert_eq!(normalize_session_key("telegram:42".to_string()), "telegram:42");
}

#[test]
fn normalize_session_key_defaults_to_gui() {
    assert_eq!(normalize_session_key("abc".to_string()), "gui:abc");
}
```

对 command handler，可创建测试 channel：

```rust
#[tokio::test]
async fn stop_chat_handler_returns_error_when_manager_channel_closed() {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);
    let state = AppState::for_test(tx);

    let Json(value) = stop_chat_handler(State(state), Json(StopChatRequest::default())).await;
    assert_eq!(value["status"], "error");
}
```

对 router 层，建议覆盖：

- `/api/chat` 普通消息发送后返回 SSE。
- `/api/chat` message 为 `/stop` 时走 stop 分支。
- session history/delete 对裸 id 自动补 `gui:`。
- upload file 缺少 file/body/channel 时返回明确错误。
- cron/MCP handler 的成功、service error、oneshot dropped 三类响应。

## 验证方法

执行：

```powershell
cargo test -p agent-diva-manager handlers
cargo test -p agent-diva-manager
just fmt-check
just check
```

预期结果：

- `agent-diva-manager/src/handlers.rs` 或相邻 `handlers` 模块存在测试。
- 覆盖主要 JSON shape：成功返回 `{"status":"ok"}`，失败返回 `{"status":"error","message":...}`。
- 覆盖 SSE event name：`delta`、`reasoning_delta`、`tool_start`、`tool_finish`、`final`、`error`。
- 覆盖 manager channel 关闭和 oneshot 接收失败路径。

## 优先级

P3
