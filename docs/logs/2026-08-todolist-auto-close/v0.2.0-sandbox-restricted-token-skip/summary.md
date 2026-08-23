# Summary — SANDBOX-WINDOWS-RESTRICTED-TOKEN-ENV

- 版本：`v0.2.0-sandbox-restricted-token-skip`
- 日期：2026-08-23
- 类型：测试环境跳过（保留能力覆盖）

## 背景

本机 `just test` 中 `test_executor_creation` 与 `test_restricted_token_execution`
因 Restricted Token 不可用失败。生产 executor 行为正确：`is_available()` 已探测
`CreateRestrictedToken`。

## 做了什么

- 测试侧先探测 `is_available()`；不可用则 `eprintln!` 后 return（仍计为 pass，不是
  `#[ignore]`）。
- 可用环境继续断言 token 创建与 `echo hello` 执行。
- 未改生产 `WindowsSandboxExecutor` / `create_restricted_token` / `execute`。

## 影响范围

- `agent-diva-sandbox/src/platform/windows.rs` 测试模块
- `TODOLIST.md`
- 本日志
