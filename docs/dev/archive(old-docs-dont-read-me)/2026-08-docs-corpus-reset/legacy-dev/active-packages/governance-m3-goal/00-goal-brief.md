# M3 Goal Brief

## 目标与完成定义

关闭 `TODOLIST.md` 中的 GMH-30B2、GMH-31、GMH-32、GMH-33，并以自动化证据和
隔离 profile 的桌面验收关闭 M3。执行顺序固定为：

```text
基线与纳入债务 → GMH-30B2 → GMH-31 → GMH-32 → GMH-33 → M3 验收
```

每个 GMH 阶段必须完成实现、成功与 failure-path 测试、`docs/logs` 四件套、
`TODOLIST.md` 同步和聚焦 Conventional Commit，然后按 `06-goal-checkpoints.md`
报告自动化证据并继续下一阶段。中间阶段不要求人工 smoke；所有人工桌面/CLI smoke
统一在最终 Epic/M3 验收执行。若需要真实 provider、真实 key、真实用户 profile、管理员
权限或系统安全策略变更，仍必须暂停并取得明确授权。

## 纳入范围

- Plan、Sandbox、Memory 共享 core governance coordinator 和 `.laputa/governance.db`；
- Plan/Memory receipt 消费、取消、超时、并发、重启与 crash-window 恢复；
- 统一 Manager approval HTTP/SSE/Tauri contract 与 typed reason code；
- GUI 全局审批抽屉、badge、三域就地审批和 Memory edit-and-approve；
- CLI 交互审批、headless 默认拒绝及显式 durable queue；
- core/Manager Rust 1.94 all-target Clippy、Manager log-range flake、Windows release
  EXE access denied 四项已选债务；
- 最终 Rust、GUI、CLI、Manager、Tauri 与隔离桌面 smoke。

## 明确边界

- 不修改真实用户 profile，不读取真实 key，不调用真实 provider，不 push；
- 不引入第二套 ledger、runtime、Manager、store、Tool Gateway 或长期双写；
- 不持久化 raw command、cwd、reason、完整 prompt、Memory patch、附件正文或 secret；
- 不自动重放命令或 outcome unknown 的副作用；
- 不新增 `/v1`，保持现有 HTTP path/method/JSON、Tauri command 和 SSE 兼容；
- 不自动关闭安全软件、添加排除项或修改系统/企业安全策略；
- GMH-41、GMH-50、GMH-52、GMH-53 不是本 Goal 的功能范围，除非 M3 验收依赖。

## 交付分段

1. 基线：建立 contract fixture，单独修复纳入债务中的 Clippy 与 log flake；
2. GMH-30B2：Plan/Memory 使用统一 coordinator 和恢复状态机；
3. GMH-31：统一 query/detail/decision/cancel/events 服务和兼容适配器；
4. GMH-32：统一 GUI projection、抽屉/badge、就地审批和隔离桌面观察；
5. GMH-33：interactive/headless/queue 与三域 E2E；
6. 最终：Windows release 启动修复、全量门禁、桌面验收与 M3 关闭。

## `/goal` 启动文本

```text
/goal 完成 agent-diva 的 M3 HITL 闭环（GMH-30B2、GMH-31、GMH-32、GMH-33）及已纳入的 core/Manager all-target Clippy、Manager log-range flake、Windows release EXE access 债务。开始前完整读取并严格执行 docs/dev/governance-m3-goal/ 下全部文档、根 AGENTS.md、TODOLIST.md 和 LOCK.md。按基线→GMH-30B2→31→32→33→最终验收推进；每阶段完成实现、failure-path tests、docs/logs 四件套、TODOLIST 同步和聚焦 Conventional Commit，报告提交 SHA、自动化证据与剩余风险后继续下一阶段；中间阶段不要求人工 smoke，所有人工桌面/CLI smoke 统一推迟到最终 Epic/M3 验收。保持现有 API/Tauri wire compatibility，不新增第二套 authority/store/runtime，不保存命令、prompt、Memory patch 或 secret，不自动重放未知副作用。Plan/Memory 重启恢复 Pending，撤销无 prepared journal 的 Allowed；prepared+Consumed 仅幂等恢复。GUI 使用全局审批抽屉+就地卡片共享同一 server projection。CLI/headless 默认 fail-closed，仅显式 queue 且 Manager 可用时排队。使用隔离 profile，禁止真实 provider、真实 key、push 和自动修改系统安全策略；若需要真实 provider/key/profile、管理员权限或系统安全策略变更必须暂停授权。最终通过资料包规定的全部自动化门禁与集中人工冒烟，关闭所有 M3 条目和纳入债务后才可标记 Goal complete。
```
