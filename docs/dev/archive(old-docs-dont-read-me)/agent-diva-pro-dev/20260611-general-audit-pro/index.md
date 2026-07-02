# agent-diva-pro 通用审计问题索引

本目录记录 2026-06-11 针对 agent-diva-pro 分支的通用审计问题说明。文档基于 `.hermes/audit/final-report.md`、`.hermes/audit/branch-ownership.md`、相关 batch 审计报告和当前 pro 分支源码只读分析撰写。

| 优先级 | 编号 | 问题 | 文档 | 归属 |
| --- | --- | --- | --- | --- |
| P0 | P0-1 | Windows 沙箱未实现 | [P0-1-windows-sandbox.md](P0-1-windows-sandbox.md) | pro-only |
| P1 | P1-5 | 子代理无 timeout/并发上限 | [P1-5-subagent-timeout.md](P1-5-subagent-timeout.md) | both |
| P1 | P1-8 | SQLite 未启用 foreign_keys/WAL | [P1-8-sqlite-wal.md](P1-8-sqlite-wal.md) | pro-only |
| P2 | P2-9 | Tool trait 双定义 | [P2-9-tool-trait-unify.md](P2-9-tool-trait-unify.md) | pro-only |
| P2 | P2-10 | 热路径 clone + 同步 IO | [P2-10-clone-sync-io.md](P2-10-clone-sync-io.md) | both |
| P3 | P3-12 | clippy 不通过 | [P3-12-clippy-cleanup.md](P3-12-clippy-cleanup.md) | pro-only |

## 优先级说明

P0-1 属于安全边界缺失：Windows 配置为沙箱模式时没有真实 OS 隔离，审批后还可能进入无沙箱执行路径。

P1-5 属于并发和资源可控性问题：pro 已为批量隔离子代理增加部分 timeout，但后台子代理没有整体 timeout/取消 API，批量任务也没有并发上限。

P1-8 属于数据一致性和本地数据库并发稳定性问题：schema 声明了外键级联，但连接未启用 `foreign_keys`，也缺少 WAL 和 busy timeout。

P2-9 属于架构和维护问题：`agent-diva-tools` 与 `agent-diva-tooling` 同时定义 Tool 抽象，可能造成工具注册和 trait object 分裂。

P2-10 属于性能和异步运行时稳定性问题：pro 的 agent loop 扩展后 clone 热点更多，`MemoryManager` 仍在 async-facing 路径中执行同步文件 IO。

P3-12 属于工程卫生问题：clippy warning 在 `-D warnings` 策略下会阻塞验证，但修复风险低。

## 建议处理顺序

1. 先修复 P0-1，明确 Windows 沙箱真实能力和失败时的降级策略。
2. 再修复 P1-5，为 pro 子代理补齐整体 timeout、取消 API 和并发上限。
3. 随后修复 P1-8，保证 planning store 的引用完整性和 SQLite 并发行为。
4. 再处理 P2-9，统一 Tool trait，降低后续工具开发分裂风险。
5. 接着处理 P2-10，降低长会话和高频工具调用下的 clone/IO 成本。
6. 最后清理 P3-12，使 `just check` 回到可通过状态。
