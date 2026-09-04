# C6-D typed AgentLoop Clean Break

日期：2026-09-04

## 结果

C6-D 完成 strict Clean Break。AgentLoop 的生产和 direct/test/CLI seam 现在直接以
`ChannelEnvelopeV1` admission，并以 `Option<ChannelCommand>` 表达 adapter egress；旧
message DTO、旧 MessageBus message queues、receiver/take/publish API、callback subscriber
和旧 metadata 暗道不再存在于 active source。没有兼容桥、dual bus、fallback 或 owned
compatibility DTO。

信任边界固定为：OwnerFrontend message 必须携带 `OwnerTurnContextV1`；ExternalUser 和
Runtime message 必须没有 owner context。OwnerFrontend 的最终结果只进入 AgentEvent
projection；ExternalUser/Runtime 的结果保留 typed address、session、request、trace、thread
和 inbound message 的 `reply_to`，再进入 Manager-owned bounded adapter egress。

## 主要改动

- Core 收窄为 AgentEventBus，删除旧 message queue 与 API；新增 trust-matrix validation、
  identity-only user activity 和确定性的 typed content rendering。
- AgentLoop、runtime control、session worker 和 dispatcher 贯通 typed envelope；显式 session
  identity、correlation、stop/reset、worker recovery 和 bounded admission 保持有效。
- Manager `/api/runtime/turns` 保持 HTTP/SSE wire 兼容，在 admission 前通过共享 FileManager
  解析附件；Cron/Subagent 通过 cloneable FabricHandle 注入 Runtime-origin envelope。
- 六个 external adapter 继续 Fabric-first；message tool 使用 typed ChannelCommand callback；
  Email subject、图像/音频/视频/文件/位置/card/reference 等 typed parts 不降级为匿名 map。
- supervised run store 以 additive SQLite schema migration 增加 nullable `context` 列；打开旧表
  时通过 `PRAGMA table_info` 检测并补列，旧行保持不变且没有 typed route 的记录在 Subagent
  handler 处 fail-closed。context/tags 序列化错误显式传播，不再静默写入空值。
- 最终审查修复了 opaque parent session 的 Subagent result route inheritance、FileManager
  `sha256:` 前缀规范化、production admission panic 分支，以及 clean-break checker 对全部
  active workspace crates 的覆盖。
- clean-break checker 递归检查 active product roots，并排除历史/archive 文档、构建输出与
  checker 自身，防止旧 token/API 回流。

## 隔离与发布边界

- Worktree：`C:\Users\Administrator\Desktop\morediva\agent-diva-c6-d-dto-cleanbreak`
- Branch：`feat/c6-delete-legacy-dto`
- 本分支未 push，未 merge 到 `dev`；C6-E（真实平台、桌面和全工作区 MSRV）保持 open。

实现及验证提交见交付报告与 `git log`；本迭代的验收记录在同目录的
`verification.md`、`release.md` 和 `acceptance.md`。
