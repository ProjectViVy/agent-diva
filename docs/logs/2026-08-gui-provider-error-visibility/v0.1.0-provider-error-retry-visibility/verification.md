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

## 人工 smoke（已执行部分）

执行环境：隔离 config（`gateway.port=3111`、`deepseek.api_base → 127.0.0.1:3110`），
独立 `target-smoke` 构建（用户正在运行的 `agent-diva.exe gateway run`（PID 8856）
占用 `target\debug` 与 3000 端口，未触碰）。

1. **重试链路（CLI 非交互，真实运行时）**：本地 mock 服务器对每个请求返回 500，
   `agent-diva agent --message "hi"`（隔离 config）：
   - mock 请求计数 = **4**（1 次初始 + 3 次重试，与 `MAX_RETRIES=3` 一致）；
   - 最终 CLI 输出 `Error: Failed to process message: API error: HTTP 500
     Internal Server Error: {"error":"simulated server error"}`——错误正确传播；
   - 无挂死（约 10s 内完成，含 1s/2s/4s 退避）。
2. **HTTP SSE 端到端（未完成）**：隔离 gateway 因 CLI 硬编码
   `DEFAULT_GATEWAY_PORT=3000`（`build_gateway_runtime_config` 忽略
   `config.gateway.port`）与用户 gateway 端口冲突，无法在隔离端口起第二个
   实例——HTTP/SSE 层的 `provider_retry` 事件观察留待 GUI 实机验收
   （`acceptance.md` 场景 A）。该缺口已记 TODOLIST
   `GATEWAY-PORT-CONFIG-IGNORED`。
3. **GUI 实机（待执行）**：`smoke-test-required-for-user-visible-change` +
   `gui-changes-need-gui-smoke`——需真实 Tauri 桌面（当前会话仅自动化验证）。

## 观察点（实机时记录）

- 徽标出现时机与 attempt 计数是否与日志 `retry.rs` warn 一致；
- 断流后错误气泡文案与 `isTyping` 恢复；
- 长时间工具执行（60-120s）仅出现一次 stall 提示且最终结果正常送达。
