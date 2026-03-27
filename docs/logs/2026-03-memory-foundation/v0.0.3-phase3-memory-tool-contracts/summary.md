# v0.0.3 phase3 memory tool contracts

## 本次变更

- 在 `agent-diva-memory` 中补齐 Phase 3 所需的 tool-facing service adapter，新增 `WorkspaceMemoryService`。
- 统一 memory tool contract 命名，明确区分：
  - `MemoryToolRecallResult`
  - `DiaryToolReadResult`
  - `DiaryToolListResult`
- 为 `WorkspaceMemoryService` 提供最小可用 recall 行为：
  - 扫描 diary 文件并按 query / domain / scope / time 做基础过滤
  - 保留 `MEMORY.md` 兼容层作为 recall fallback
- 在 `agent-diva-tools` 中新增正式工具：
  - `memory_recall`
  - `diary_read`
  - `diary_list`
- 在 `agent-diva-agent` 的 tool registry 注册上述记忆工具，使后续 recall-before-answer policy 可以只依赖工具层入口。

## 分层结果

- `agent-diva-core` 继续只保留 `MEMORY.md` / `HISTORY.md` / `DailyNote` 最小兼容能力。
- `agent-diva-memory` 成为增强记忆查询与 tool contract 的唯一权威层。
- `agent-diva-tools` 现在只消费 `agent-diva-memory` contract，不再从 `core::memory` 获取任何增强类型。
