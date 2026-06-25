# LOCK

Codex/Cursor/人工协作并行开发互斥锁。

本文档的目标不是记录长期计划，而是声明“当前谁正在改什么”，避免多个并行会话直接改到同一批文件。

## Status

- Lock State: `FREE`
- Scope: `none`
- Owner: `none`
- Session/Task: `none`
- Branch/Worktree: `none`
- Started At: `none`
- Last Heartbeat: `none`
- Expires At: `none`

## Lock Rules

1. 在任何写文件、运行会修改工作区的命令、或准备提交之前，先阅读本文档。
2. 如果 `Lock State` 是 `HELD`，并且 `Last Heartbeat` 仍在有效期内，后续会话不得修改 `Scope` 覆盖的文件。
3. 如果新任务必须并行推进，优先创建独立 `git worktree`/分支；即便如此，也要在这里登记自己的锁定范围。
4. `Scope` 必须写明文件、目录或模块，不允许只写“修 bug”“做功能”这种模糊描述。
5. 持锁会话至少每 30 分钟刷新一次 `Last Heartbeat`，离开前必须释放锁，或把状态改成 `STALE` 并写清原因。
6. 如果发现锁过期，接手者先在 `Handoff Notes` 记录观察，再更新 `Status` 并接管，避免静默覆盖。
7. 如果需要阻止任何并行写入，把 `Scope` 设为 `GLOBAL`；仅在大范围重构、迁移、批量格式化时允许这样做。

## Acquisition Checklist

- 把 `Lock State` 改为 `HELD`
- 填写 `Owner`、`Session/Task`、`Branch/Worktree`
- 填写精确的 `Scope`
- 记录 `Started At`、`Last Heartbeat`、`Expires At`
- 如为并行任务，补充与其他任务的边界说明

## Release Checklist

- 确认本次修改已完成、移交、或明确暂停
- 把 `Lock State` 改为 `FREE`
- 把 `Scope`、`Owner`、`Session/Task` 清空为 `none`
- 在 `Handoff Notes` 记录剩余风险、阻塞或下一步

## Active Lock

在这里填写当前唯一有效锁。没有活跃任务时保持默认值。

## Handoff Notes

- `2026-06-25T20:01:15.4363429+08:00`: Codex claimed the lock for MCP C-2 `write()` serialization fix in `agent-diva-tools/src/mcp_sdk.rs`.
- `2026-06-25T20:06:57.7482366+08:00`: Codex completed MCP C-2 parallel tool call fix, added regression coverage, ran `cargo test -p agent-diva-tools`, and released the lock.
