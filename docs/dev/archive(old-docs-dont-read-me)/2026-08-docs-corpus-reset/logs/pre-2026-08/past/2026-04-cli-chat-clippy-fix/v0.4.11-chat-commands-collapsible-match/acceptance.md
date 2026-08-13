# Acceptance: v0.4.11-chat-commands-collapsible-match

## 验收步骤

1. 在仓库根目录运行 `just check`
2. 确认输出中不再出现 `agent-diva-cli/src/chat_commands.rs` 的 `collapsible_match` 报错

## 验收标准

- CI 阻塞错误消失
- CLI 事件输出逻辑保持原行为
