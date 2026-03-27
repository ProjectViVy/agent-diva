# Summary

## 变更概述

- 将 prompt 主记忆路径从 `MEMORY.md` 全量注入切换为 turn 级 compact recall 注入。
- 在 `agent-diva-memory` 中补齐 snapshot/hydrate 灾后恢复链路，并在初始化时自动回填 diary 与 `MEMORY.md` chunks。
- 新增 embedding abstraction、SQLite embedding cache、hybrid-ready recall 模式。
- 新增 `memory_search` 与 `memory_get` 两个只读 memory tool，保留现有 `memory_recall` / `diary_read` / `diary_list` contract。

## 影响范围

- `agent-diva-agent` 的 system prompt 组装与默认工具说明。
- `agent-diva-memory` 的 SQLite schema、恢复逻辑、检索编排与快照文件。
- `agent-diva-tools` 的 memory tool 集合与参数 schema。
