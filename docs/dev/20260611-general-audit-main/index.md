# agent-diva main 分支通用审计问题索引

本文档索引基于 2026-06-11 对 main 分支源码的只读分析，覆盖从 agent-diva-pro 审计报告中归属为 main-first 的 9 个问题，以及需要 main/pro 两边各自修复的 both 类 2 个问题。

| 编号 | 优先级 | 文档 | 摘要 |
|---|---|---|---|
| P0-2 | P0 | [P0-2-shell-security.md](P0-2-shell-security.md) | Shell 工具与 AgentLoop 默认 `restrict_to_workspace = false`，安全基线 fail open。 |
| P0-3 | P0 | [P0-3-atomic-persistence.md](P0-3-atomic-persistence.md) | Config/Memory 直接覆盖写入；Session 已部分原子但缺目录 fsync、唯一临时名和跨进程锁。 |
| P1-4 | P1 | [P1-4-channel-manager-lock.md](P1-4-channel-manager-lock.md) | ChannelManager 在持有全局 handlers 锁时 await start/stop/send/update。 |
| P1-5 | P1 | [P1-5-subagent-timeout.md](P1-5-subagent-timeout.md) | main 已有默认并发上限和循环级 timeout，但缺强制取消、abort API、join 观测和 running map 原子注册。 |
| P1-6 | P1 | [P1-6-telegram-allow-from.md](P1-6-telegram-allow-from.md) | Telegram `allow_from` 为空时默认允许所有用户。 |
| P1-7 | P1 | [P1-7-s3-todo.md](P1-7-s3-todo.md) | S3 storage backend 占位实现含 6 个 `todo!()`，启用后会 panic。 |
| P2-10 | P2 | [P2-10-clone-sync-io.md](P2-10-clone-sync-io.md) | main 主循环仍有事件/debug/tool result clone 热点；默认 MemoryManager async trait 内执行同步文件 IO。 |
| P3-11 | P3 | [P3-11-app-vue-refactor.md](P3-11-app-vue-refactor.md) | `App.vue` 约 1384 行，混合状态、API、stream、session、配置和顶层 UI 编排。 |
| P3-13 | P3 | [P3-13-handlers-testing.md](P3-13-handlers-testing.md) | `agent-diva-manager/src/handlers.rs` 约 880 行控制面 handler 无测试覆盖。 |
| P3-14 | P3 | [P3-14-litellm-split.md](P3-14-litellm-split.md) | `litellm.rs` 约 1601 行，DTO、request、response、stream、HTTP、模型解析集中在单文件。 |
| P3-15 | P3 | [P3-15-provider-error-structured.md](P3-15-provider-error-structured.md) | `ProviderError::ApiError(String)` 丢失 status、provider、model、code 等结构化信息。 |

## 建议处理顺序

1. P0-2 Shell 默认安全限制：先建立工具执行安全基线。
2. P0-3 原子持久化：统一 config/memory/session 的可靠写入策略。
3. P1-6 Telegram 默认拒绝：避免公开 bot 的未授权访问。
4. P1-4 ChannelManager 锁粒度：降低控制面阻塞和死锁风险。
5. P1-5 子代理 timeout/取消治理：补齐强制 timeout、abort API 和任务 registry。
6. P1-7 S3 todo：移除 panic 占位或完成后端实现。
7. P2-10 clone + 同步 IO：降低主循环事件复制和 MemoryManager 阻塞 IO。
8. P3-15 ProviderError 结构化：为 provider 错误处理和 UI 提示打基础。
9. P3-14 LiteLLM 拆分：在错误结构化后拆 provider 文件更顺。
10. P3-13 handlers 测试：为控制面后续拆分和修复建立回归保护。
11. P3-11 App.vue 拆分：最后做 GUI 结构性整理，降低与功能修复冲突。

## 验证总纲

每个问题文档包含独立验证命令。完成任一修复后，至少执行：

```powershell
just fmt-check
just check
```

涉及行为修复时补充对应 crate 测试；涉及 GUI 的 P3-11 需额外执行 GUI typecheck/build 和一次最小 smoke。
