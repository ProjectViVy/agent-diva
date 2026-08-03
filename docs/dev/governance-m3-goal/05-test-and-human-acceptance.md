# 测试与人工验收合同

## 自动化矩阵

每个 domain 至少覆盖：submit payload-free、Allow/Deny 首胜、重复相同响应、同键异操作、
stale version、timeout、cancel、digest 篡改、receipt 失配、重复 consume、persistence failure、
restart Pending、restart dangling Allowed、prepared+Consumed 恢复。

额外场景：

- Plan：revision edit 撤销旧 request；prepared context 唯一；TODO 不重复 materialize；
- Memory：proposal edit 撤销旧 digest；missing target/schema conflict；apply/changelog exactly once；
- Command：ApproveOnce 的同一真实 `echo` 仅执行一次；reject/expire/restart 不执行；
- Manager：route/DTO/status fixture；typed error；SSE 顺序、lag、cursor reconnect、去重；
- Tauri/TS：Rust fixture 可解析；旧 command contract 不漂移；
- GUI：badge/抽屉/就地卡同步、stale、过期、断线、refresh failure、键盘行为；
- CLI：interactive allow/deny、headless fail、queue success/unavailable、JSON/exit code。

## 阶段验证

每阶段运行受影响 crate/tests 和最小真实路径；最终至少运行：

```text
cargo clippy -p agent-diva-core --all-targets -- -D warnings
cargo clippy -p agent-diva-manager --all-targets -- -D warnings
cargo test -p agent-diva-core
cargo test -p agent-diva-sandbox
cargo test -p agent-diva-laputa
cargo test -p agent-diva-manager
cargo test -p agent-diva-cli
cargo test -p agent-diva-gui
cd agent-diva-gui && npm test
cd agent-diva-gui && npm run build
just fmt-check
just check
just test
```

测试名若移动，必须在同一提交更新本合同和 iteration `verification.md`。

## 桌面人工矩阵

本矩阵只在 GMH-30B2、31、32、33 全部实现和自动化门禁完成后的最终 Epic/M3 验收
集中执行。中间阶段不得要求用户进行人工 smoke，也不得因尚未执行本矩阵而阻塞后续
GMH 实现；但自动化测试不能冒充最终人工观察。

在隔离 profile、无真实 provider 下观察：

1. debug 外部 gateway 与 release embedded gateway 均能启动/退出，无孤儿进程和端口；
2. 三域 Pending 同时出现，badge 数量、抽屉和就地卡一致；
3. Command allow once 只执行一次，deny/expire 不执行；
4. Plan allow 后只创建一个 execution，deny 保持 AwaitingApproval/未执行；
5. Memory edit 后旧审批失效，新审批可 allow-only/allow-and-apply 且只 apply 一次；
6. 两客户端同时响应时仅一个成功，另一端显示 stale/conflict 并刷新；
7. 断线重连不重复卡片；Manager 重启恢复 Plan/Memory Pending、撤销 Command；
8. committed-but-refresh-failed 不自动重复 mutation；
9. 键盘可完成筛选、展开、批准/拒绝，状态不只依赖颜色。

人工在 `06-goal-checkpoints.md` 或当期 acceptance evidence 中记录日期、构建 SHA、profile、
观察结果和失败截图/日志位置。真实 provider 路径不属于本 M3 Goal。

## Windows release access

先只读检查 ACL、Zone.Identifier、占用进程、hard-link/产物状态和 Windows/安全软件事件。
可通过重新构建、复制到隔离 artifact 目录或修复项目内产物流程解决。若需要管理员 ACL、
Defender/EDR 排除或企业策略变化，必须暂停并取得明确人工授权；不得自动弱化系统安全。

## 最终完成判定

- GMH-30B2、31、32、33 及四项纳入债务全部关闭；
- 全部门禁与桌面矩阵通过，无“自动化通过代替人工观察”；
- 无 sensitive payload 进入 ledger/log/fixture；
- 每个 story 有四件套、回滚说明和聚焦提交；
- `LOCK.md` 已释放，无临时脚本、测试 profile、端口或进程残留；
- 未 push，未调用 provider，未读取真实 key。
