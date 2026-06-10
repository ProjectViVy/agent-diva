# P1-6: Telegram 默认允许所有用户

## 问题描述

`agent-diva-channels/src/telegram.rs` 的 `TelegramHandler::is_allowed` 在 `allow_from` 为空时直接返回 `true`：

```rust
fn is_allowed(&self, sender_id: &str) -> bool {
    if self.allow_from.is_empty() {
        return true;
    }
    // ...
}
```

轮询消息处理闭包中也重复实现了相同逻辑：

```rust
let is_allowed = allow_from.is_empty()
    || allow_from.contains(&sender_id)
    || (sender_id.contains('|')
        && sender_id
            .split('|')
            .any(|p| allow_from.contains(&p.to_string())));
```

测试 `test_telegram_handler_is_allowed_empty_list` 还把空 allowlist 的放行行为固化为预期：

```rust
assert!(handler.is_allowed("anyone"));
assert!(handler.is_allowed("12345"));
```

这意味着只要配置了 Telegram token 并启用 channel，若未配置 `allow_from`，任意 Telegram 用户或群成员都可以与 agent 交互。

## 影响评估

- 安全影响：Telegram bot token 泄漏、bot 被搜索到、被加入群聊或被陌生用户私聊时，会默认开放 agent 能力。
- 数据泄漏：未授权用户可能通过对话触发上下文、工具、记忆或文件相关能力，间接获得敏感信息。
- 资源滥用：攻击者可持续发送消息消耗 LLM token、网络请求和本地计算资源。
- 误配置风险：用户通常会认为空白名单代表“尚未配置访问”，但当前行为是 fail open。

## 解决方案

将 Telegram 权限默认改为 fail closed：`allow_from` 为空时拒绝普通消息，并在启动时输出明确告警。若确实需要公开 bot，应增加显式开关，例如 `allow_all = true`。

示例修复：

```rust
fn is_allowed(&self, sender_id: &str) -> bool {
    if self.allow_from.is_empty() {
        return false;
    }

    self.allow_from.contains(&sender_id.to_string())
        || sender_id
            .split('|')
            .any(|part| !part.is_empty() && self.allow_from.contains(&part.to_string()))
}
```

轮询闭包应复用同一逻辑，避免两处权限判断漂移。可以抽取纯函数：

```rust
fn sender_allowed(allow_from: &[String], sender_id: &str) -> bool {
    if allow_from.is_empty() {
        return false;
    }
    allow_from.contains(&sender_id.to_string())
        || sender_id
            .split('|')
            .any(|part| !part.is_empty() && allow_from.contains(&part.to_string()))
}
```

如需兼容公开 bot：

```rust
if config.allow_all {
    return true;
}
```

但 `allow_all` 必须是显式配置，不能由空列表隐式表达。

## 验证方法

执行：

```powershell
cargo test -p agent-diva-channels telegram
just fmt-check
just check
```

应调整并新增测试：

- `allow_from = []` 时，`is_allowed("anyone") == false`。
- `allow_from = ["12345"]` 时，`is_allowed("12345") == true`。
- `allow_from = ["12345"]` 时，`is_allowed("12345|username") == true`。
- `allow_from = ["username"]` 时，`is_allowed("12345|username") == true`。
- 轮询闭包与 `TelegramHandler::is_allowed` 使用同一判断函数。

## 优先级

P1
