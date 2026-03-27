# v0.0.2 agent-diva-memory crate split

## 记录时间

- `2026-03-26 15:55:38 CST`

## 本次变更

- 新增 `agent-diva-memory` crate，承接增强记忆子系统的统一实现入口。
- 将 `MemoryDomain`、`DiaryPartition`、`MemoryScope`、`DiaryEntry`、`MemoryRecord`、`MemoryQuery`、`DiaryFilter`、`MemoryStore`、`DiaryStore`、`RecallEngine`、`MemoryToolContract`、`DiaryToolContract`、`FileDiaryStore` 从 `agent-diva-core` 迁出。
- 将 `memory/diary/rational/YYYY-MM-DD.md`、`memory/diary/emotional/`、`memory/index/` 的路径布局迁移到 `agent-diva-memory`。
- `agent-diva-agent` 直接依赖 `agent-diva-memory`，理性日记提取策略继续保留在 agent runtime。
- `agent-diva-tools` 已切换为依赖 `agent-diva-memory` 的 contract 与 service，memory tools 不再从 `core` 获取增强类型。
- `agent-diva-core::memory` 收缩为最小兼容层，仅保留 `MemoryManager`、`Memory`、`DailyNote` 和 `MEMORY.md` / `HISTORY.md` / 每日日志基础读写。

## 影响范围

- 记忆系统增强能力不再压在 core 中，后续 SQLite、FTS、embedding、hybrid retrieval 可以只在 `agent-diva-memory` 内演进。
- 兼容链路保持不变，`ContextBuilder` 继续只读取 `MEMORY.md`，不会全量注入 diary。
- 本次迁移不保留重复定义、shim 或长期 re-export，增强记忆类型在工作区内只保留一份权威实现。
- 依赖方向已调整为：
  `agent-diva-memory -> agent-diva-core`
  `agent-diva-agent -> agent-diva-memory + agent-diva-core`
  `agent-diva-tools -> agent-diva-memory + agent-diva-core`
