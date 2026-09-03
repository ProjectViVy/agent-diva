# C6-D acceptance

日期：2026-09-04

## 产品/架构验收

- `ChannelEnvelopeV1` 是 AgentLoop admission 的唯一入站合同，直接产生
  `Option<ChannelCommand>`；没有旧 DTO、dual bus、兼容别名或 fallback。
- trust matrix 可观察且 fail-closed：OwnerFrontend 缺 context 拒绝；ExternalUser/Runtime
  携带 owner context 拒绝；ExternalUser 与 Runtime 的 context-free message 使用固定 Agent
  semantics。
- typed correlation 使用 envelope 的 session/request/trace/message/thread/sequence 权威字段；
  外部/runtime response 保留 session/request/trace/thread，并将入站 message ID 映射为
  `reply_to`。OwnerFrontend 最终结果只进入事件/projection，不尝试不存在的 adapter。
- text/markdown 稳定拼接；location/card/reference 有稳定表达；Email subject 保留；附件在
  Manager admission 前经共享 FileManager/attachment authority 校验，非法引用 fail-closed；
  image/audio/video/file 保持 typed `AttachmentRef` parts。
- Cron 和 Subagent 通过 cloneable FabricHandle 产生 Runtime-origin ingress；六个 external
  adapter 继续 Fabric-first；Manager-owned egress 独立 bounded，ChannelRuntime 负责 pacing。
- Presence 只消费 identity-only user activity/poke，不抢占单一 turn receiver，也不接触消息正文。
- stop/reset、same-session FIFO、cross-session concurrency、queue/backpressure、control
  priority、shutdown drain、worker recovery 和 typed runtime control 均有回归测试。
- Manager `/api/runtime/turns` 的 HTTP/SSE wire contract 保持可用；CLI minimum help smoke 与
  typed update-plan SSE integration 通过。

## 交付者与审查者操作

1. 在本分支运行 `just fmt-check && just check && just test`。
2. 运行 `just channel-clean-break-check`，确认 active source zero-hit proof。
3. 阅读 `verification.md` 中 Rust 1.80 probe 的 C6-E 限制；不要把本分支标记为完整 C6。
4. 审查提交后，再由父任务决定是否合入 `dev`；本次交付本身不 merge、不 push。

## 关闭条件

C6-D 已满足并在 `TODOLIST.md` 标记 Done。C6 顶层和 C6-E 不关闭，直到真实平台 receipt、
切换后桌面 smoke、以及完整 Rust 1.80/MSRV acceptance 有独立证据。
