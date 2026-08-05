# Verification — GUI Provider 错误/重试可见性修复

- 版本：`v0.1.0-provider-error-retry-visibility`
- 日期：2026-08-06

## 自动化验证

| 门 | 命令 | 结果 |
|----|------|------|
| 格式化 | `just fmt-check` | 通过（先 `cargo fmt` 修正本迭代新增代码） |
| Clippy | `just check`（`cargo clippy --all -- -D warnings`） | 通过 |
| core | `cargo test -p agent-diva-core` | 678 通过（含 2 个新 serde roundtrip） |
| providers | `cargo test -p agent-diva-providers` | 127 通过（含 2 个新 retry 回调测试；本地 TCP mock + `no_proxy` 防系统代理 502） |
| agent | `cargo test -p agent-diva-agent` | 372 通过（含新 `provider_retry_attempt_emits_bus_event`） |
| manager | `cargo test -p agent-diva-manager` | 109 通过（含 3 个新 `forward_chat_events` 测试，`start_paused` 时间推进） |
| e2e | `cargo test -p agent-diva-e2e` | 61 通过 |
| GUI | `npm run test`（vitest） | 59 files / 454 tests 通过（含 3 个新 ChatView 徽标渲染测试） |
| GUI 类型 | `npx vue-tsc --noEmit` | 通过 |

说明：`cargo clippy --all --all-targets` 会命中预存在的
`examples/minimax_sync_tts.rs` `useless_conversion`（Rust 1.94 新 lint，
该文件本迭代未改动；`just check`/`just ci` 不编译 examples，不受影响），
已记 TODOLIST。

## 人工 smoke（待实机执行）

`smoke-test-required-for-user-visible-change` + `gui-changes-need-gui-smoke`：

1. **错误路径（复现原 bug 场景）**：把默认 Provider 的 api_base 指向一个不可达
   地址（或断网），在 GUI 发送消息：
   - 预期：约 1s/2s/4s 重试间隔内，流式消息上出现「未响应，重试 (1/3)」徽标，
     随后 (2/3)、(3/3) 更新；
   - 重试耗尽后（或 60s 无事件时）出现「仍在等待 Provider 响应」提示；
   - 最终出现明确错误气泡（重试耗尽错误，或「长时间未收到响应，连接已断开」），
     `isTyping` 恢复、可再次发送。
2. **正常路径（无回归）**：正常对话 → 无重试徽标、无错误气泡，流式输出正常。
3. **CLI 对照**：`agent-diva chat` 错误 provider 场景日志可见重试 warn，行为不受影响。

## 观察点（实机时记录）

- 徽标出现时机与 attempt 计数是否与日志 `retry.rs` warn 一致；
- 断流后错误气泡文案与 `isTyping` 恢复；
- 长时间工具执行（60-120s）仅出现一次 stall 提示且最终结果正常送达。
