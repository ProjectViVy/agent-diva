# 验收

## 本片完成

- `just run -- gateway run` / `just diva-gate` 能过 bootstrap，不再因一条坏审批退出。
- 日志里对无法重放的聚合最多 `warn`，进程继续。

## 操作步骤

1. 用当前代码重新编译并启动 gateway。
2. 确认不再出现 `ledger contains an illegal state transition` 后立刻退出。
3. 对话/审批中心可打开。

## 现场数据

`C:\Users\Administrator\.agent-diva\workspace\.laputa\governance.db`
中 `088420b7-c0d7-455b-bca8-78d9083a0843` 为双 `expired`。不必手工删库。
