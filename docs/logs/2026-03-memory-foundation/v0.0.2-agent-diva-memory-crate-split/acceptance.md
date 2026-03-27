# Acceptance

## 验收要点

- `agent-diva-memory` 已成为增强记忆系统唯一入口。
- `agent-diva-core::memory` 只保留 `MemoryManager`、`Memory`、`DailyNote` 最小兼容能力。
- `agent-diva-agent` 的理性日记写入链路已切换到 `agent-diva-memory::FileDiaryStore`。
- `agent-diva-tools` 的 `memory_recall` / `diary_read` / `diary_list` 已切换到 `agent-diva-memory` contract。
- `memory/diary/rational/YYYY-MM-DD.md` 的存储路径保持不变。
- `MEMORY.md` / `HISTORY.md` 兼容行为未回退。
- `agent-diva-core` 中不存在增强记忆类型的重复实现或 re-export。

## 已知事项

- 当前 worktree 下已完成全工作区验证，未发现阻断本次 memory 拆分交付的剩余失败项。
