# Acceptance

## User checks

1. GUI 输入 `/stop`：
   - 不作为普通用户消息发送；
   - 触发 stop 请求；
   - 当前生成被中断。
2. TUI 输入 `/stop`：
   - 本地模式中断当前会话生成；
   - 远程模式调用 `/api/chat/stop` 并中断当前会话生成。
3. Telegram 输入 `/stop` 或 `/stop@botname`：
   - 返回 stop 请求反馈；
   - 当前会话生成被中断。

## Behavioral boundary checks

- `/stop` does not clear history.
- `/stop` does not switch to a new session.
- `/new` or `/reset` semantics remain separate.
