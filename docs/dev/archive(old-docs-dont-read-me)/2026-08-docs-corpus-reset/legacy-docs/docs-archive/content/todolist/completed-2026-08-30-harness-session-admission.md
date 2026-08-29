# 完成归档：HARNESS Session Admission Epic

关闭日期：2026-08-30

本记录将已完成的 `HARNESS-SESSION-ADMISSION-BOUNDED-QUEUE` 从根目录活跃待办移入历史
归档。归档内容不再驱动实施；未来若出现新的准入问题，应重新验证并创建独立活跃事项。

## 完成范围

- [x] HQ-00：冻结 canonical session identity、dispatcher/worker 所有权、配置、稳定状态码
  和 Stop/Reset/Delete 生命周期合同。
- [x] HQ-01：实现时钟可注入的有界 FIFO admission kernel、RAII lease、超时、取消、关闭
  排空和 idle eviction。
- [x] HQ-02：Bus 与 direct 入口接入同一准入 seam，拆分显式 session worker state 和
  turn-local approval/tool/subagent 快照。
- [x] HQ-03：启用持久 per-session workers、跨 session 并发、精确 runtime control、配置
  默认值和 Manager/CLI/GUI/SSE correlation。
- [x] HQ-04：加入 request-scoped provider observers、generation-aware worker supervision、
  panic 恢复、跨入口故障注入和桌面 backpressure/error UX。
- [x] HQ-05：完成全工作区门禁、定向回归、CLI/GUI/embedded-Gateway smoke、运维指南、
  迁移/回滚说明、验收记录和 Epic 归档。

## 交付结论

- 同 session 最大执行并发为一并保持 FIFO；不同 session 可在 provider 阻塞时独立推进。
- `max_queue_depth` 仅计算 waiter；queue-full 和 wait-timeout 在 provider/tool/history/BML
  副作用之前拒绝，timeout/release race 不产生幽灵 lease。
- Stop 只取消 running turn，queued target 返回 `queued_preserved`；Reset/Delete 排空整个
  session 并在 quiescence 后清理。
- Worker panic 精确排空受影响请求并允许下一请求重建；provider retry/final-wire 观察器不会
  在并发 request 之间串 listener 或 request/trace identity。
- MessageBus 仍是 transport；PlanMode、Sandbox、Approval、session history、BML、Persona、
  Evolution 权威未迁移。

## 提交与记录

- HQ-00：`1f9aa730`、`003b5633`
- HQ-01：`5ff59b5c`、`3beefb52`
- HQ-02：`9302f8e7`、`b53c619f`
- HQ-03：`011db1f3`
- HQ-04：`2e3553fb`、`5176ac18`、`0226571e`
- HQ-05：最终文档提交记录在本轮关闭 commit。

设计、逐阶段验证和发布记录保留在：

- `docs/dev/harness-session-admission/`
- `docs/logs/2026-08-harness-session-admission/`
- `docs/logs/2026-08-harness-session-admission-planning/`

## 发布与回滚边界

- 当前成果位于本地 `dev`，未推送、未发布安装包。
- 现有配置无手工迁移；缺失 `session_admission` 时自动使用冻结默认值。
- 紧急整 Epic 回滚基线为 `8b901d4f`；回滚前应停止新流量并等待已接受 turn 静默。
- `EVENTBUS-TRAIT-HOOKS`、A2A、Neuro-Link 和频道能力合同继续作为独立活跃事项，不因
  本 Epic 关闭而自动开工。
