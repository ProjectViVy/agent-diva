# 当前状态与缺口盘点

## 已完成基础

| 域 | 已有能力 | 当前 authority |
|---|---|---|
| Core | payload-free append-only ledger、version CAS、幂等、receipt binding、分页状态读取、consume/revoke/expire | `ApprovalCoordinator` + `SqliteGovernanceLedger` |
| Sandbox | durable Pending/decision、Once 执行前消费、5 分钟 session grant、Rule receipt 前置、重启撤销 Command Pending/Allowed | core ledger；raw command 仅进程内 |
| Memory | proposal digest/evidence、proposal→request 映射、approval receipt、typed apply、apply recovery journal | `MemoryGovernanceCoordinator` 自行打开同一 DB |
| Plan | canonical plan revision/CAS、AwaitingApproval、execution context 和 TODO materialize | Planning store 的独立 approval receipt |
| Manager | Command、Plan、Laputa 各自 REST；Command requested SSE；Agent bus Plan events | 分域 handler/service |
| GUI | Plan approval card、Command approval banner、Evolution proposal decision/apply | 三套分散 projection |
| CLI | 普通 direct chat、Manager gateway 启动 | direct chat 未注入 command coordinator |

## GMH-30B2 缺口

- Memory 生产 coordinator 自行组装 ledger，而不是由 Manager composition root 注入 core
  coordinator；Plan 尚未进入 core governance ledger；
- Plan approve 当前直接写 canonical Plan approval、恢复 registry、创建 execution context；
  ledger receipt 与 execution 初始化没有统一 prepared/consume/recovery 边界；
- Memory apply 虽有 journal，但需证明 `Allowed→prepared→Consumed→applied` 的 crash window
  与 core coordinator 生命周期一致；
- Plan/Memory Pending 的启动恢复、悬空 Allowed 撤销/重新 Pending 尚未统一；
- cancel、explicit expire、重复响应和跨客户端首胜缺少三域一致测试。

## GMH-31 缺口

- `/api/command-approvals`、Plan report approve、Laputa decision/apply DTO 和错误互不统一；
- Command resolve 没有公开 expected version/idempotency key，错误仍压缩为四类；
- 只有 Command requested 流，没有 durable resolved/expired/revoked 序列和 reconnect cursor；
- Tauri/TypeScript 存在 domain-specific DTO，缺少 Rust fixture→TS guard 的统一 contract；
- detail assembler 尚不能在不污染 ledger 的前提下连接 ledger metadata 与 domain payload。

## GMH-32 缺口

- `App.vue` 分别持有 Command、Plan、Evolution mutation/in-flight/reconcile 状态；
- 没有全局 pending count、统一筛选、详情抽屉和 authoritative projection；
- Command 卡、Plan 卡、Evolution 页面没有共享 version/reason/event cursor 语义；
- stale、断线重连重复事件、committed-but-refresh-failed 的统一 UX 尚未形成；
- Memory edit-and-approve 是前后两个分散动作，缺少旧 receipt 撤销可见证据。

## GMH-33 缺口

- direct CLI chat 把 `command_approvals` 设为 `None`，无法进入交互审批；
- 没有稳定的 headless `approval_required_noninteractive` 退出语义；
- 没有显式 queue 模式、request ID/status 查询和 Manager unavailable 行为；
- 缺 shell、Plan、Memory 三条 interactive/headless/queue E2E。

## 已纳入债务与环境

- core Rust 1.94 all-target Clippy：19 个既有 test-target lint；
- Manager Rust 1.94 all-target Clippy：6 个既有 test-target lint；
- Manager log-range full-suite flake：共享日志/时间范围污染；
- Windows release EXE：构建产物存在，但本机直接启动曾返回 OS error 5；
- 当前分支 `agent-diva-pro`，GMH-30B1 三提交为 `5bad8500`、`e354a39f`、
  `1e365ad0`；开工时必须重新核对 HEAD/dirty state，不能依赖本文静态快照。
