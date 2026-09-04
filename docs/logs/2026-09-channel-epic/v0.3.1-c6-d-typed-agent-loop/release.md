# C6-D release record

日期：2026-09-04

## 发布边界

交付物位于隔离 worktree：

`C:\Users\Administrator\Desktop\morediva\agent-diva-c6-d-dto-cleanbreak`

分支为 `feat/c6-delete-legacy-dto`。本次没有 push，没有 merge 到 `dev`，也没有操作远程
仓库。父任务 `/root` 负责审查和后续是否合入；当前提交只构成可审查的 C6-D branch history。

## Lock

任务开始前已按 `LOCK.md` claim `GLOBAL` scope，并在实施期间刷新 heartbeat。交付前会将
`LOCK.md` 改为 `RELEASED`，写入最终 HEAD、验证结果和 C6-E hand-off，再提交独立 release
commit。release 后工作树必须保持 clean。

## 未发布风险

C6-E 仍 open：至少一个真实平台的 ingress/receipt、切换后桌面断线恢复，以及完整 Rust 1.80
MSRV acceptance 尚未完成；channel-scoped probe 被 `getrandom 0.4.3` 的 Edition2024 manifest
要求阻塞。`imap-proto` future-incompat warning 和没有凭据而 ignored 的 QQ live harness 已
如实记录在 verification，不作为 C6-D 失败伪装成通过，也不作为 C6-E 完成证据。

## Schema migration / rollback

`supervised_runs.context` 是 nullable additive SQLite schema migration。`RunStore` 使用
`PRAGMA table_info` 检测旧表并只在缺列时增加它；旧行不改写，缺少 typed context 的记录由
Subagent handler fail-closed。该迁移不会恢复任何旧 DTO 或 queue API，也不会自动删除新增列。
旧二进制可忽略新增列并读取原有字段，但不具备 C6-D 的 typed route/result correlation 语义；
回滚前需暂停依赖 typed context 的 queued runs，并按部署版本策略处理 NULL context。
