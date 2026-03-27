# Acceptance

## 验收要点

- `agent-diva-memory` 暴露 `WorkspaceMemoryService`，并实现 memory/diary 的 tool contract。
- `agent-diva-tools` 已存在 `memory_recall`、`diary_read`、`diary_list` 三个正式工具。
- `agent-diva-agent` 已在 tool registry 中注册上述工具。
- `ContextBuilder` 仍只注入 `MEMORY.md`，没有把 diary 文件全量塞入 prompt。
- recall 仍是最小实现，不包含 FTS、embedding 或 hybrid retrieval。

## 已知事项

- `cargo test --all` 仍受 `agent-diva-cli` 既有失败用例影响，本次未处理该问题。
