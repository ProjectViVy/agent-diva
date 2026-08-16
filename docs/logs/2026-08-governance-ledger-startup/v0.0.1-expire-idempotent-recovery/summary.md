# 审批账本重复 expire 不再阻断网关启动

- 日期：2026-08-17
- 切片：`just diva-gate` / `gateway run` 在 bootstrap 报
  `approval ledger persistence failed: ledger contains an illegal state transition`
- 分支：`agent-diva-pro`（未 push）
- 版本目录：`docs/logs/2026-08-governance-ledger-startup/v0.0.1-expire-idempotent-recovery/`

## 目标

网关启动恢复可以跳过或消化历史账本里的坏聚合，不再因为一条记录让整个进程退出。

## 原因

现场 `governance.db` 中 `088420b7-…` 的事件是
`requested → allowed → expired → expired`。

`expire()` 在派生状态已经是 Expired（TTL 推出来的）时仍追加一条 `expired`。
审批中心 `materialize_expired` 每次用不同 idempotency key 再跑一遍，就会写下
第二条 `expired`。启动时 `recover_incomplete` → `states_page` → `replay`
把第二条当成非法迁移，整页失败。

## 改动

- `derive_state`：已经是 Expired 时忽略后续 `expired`。
- `expire()`：账本里已有 `expired` 事件则直接返回，不再追加。
- `states_page`：单条 replay 失败时告警并跳过，不阻断分页/恢复。
- 单条 `state()` / decide / consume 仍 fail-closed。

## 未做

- 未改写用户机器上的 `governance.db`（重复 expire 现可重放）。
- 未重启用户正在跑的 gateway。
