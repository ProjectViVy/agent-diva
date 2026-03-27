# v0.0.4 memory recall policy

## 记录时间

- `2026-03-26 17:22:25 CST`

## 本次变更

- 在 `agent-diva-agent` 的 turn 主循环中增加保守型 `recall-before-answer` policy，只在“之前做过什么 / 最近结论 / 项目进度 / 偏好 / 承诺 / 下一步”类问题上自动预取记忆。
- 自动 recall 发生在首轮 LLM 调用前，结果直接追加到当前 turn 的 system prompt，不写回 session history，也不强制在用户正文中显式说明。
- `ContextBuilder` 增加 memory 工具使用规约，明确：
  - 历史型问题优先依赖记忆而不是猜测
  - 系统已注入 recall 上下文时优先消费该上下文
  - `memory_recall` 用于泛搜索，`diary_list` / `diary_read` 用于展开细节
- `WorkspaceMemoryService` 增加面向 policy 的轻量 helper：
  - 统一获取 top recall records
  - 稳定格式化为 prompt-friendly 摘要
  - 为 `MEMORY.md` compatibility record 补 source path
- recall 匹配从“整句 contains”提升为“最小词项归一化 + 任一显著词命中”，使“之前我们对 memory 拆分做了什么结论”这类自然问题能命中 diary / `MEMORY.md`。

## 影响范围

- `MEMORY.md` 兼容注入行为保持不变；diary 仍不全量注入 prompt。
- `agent-diva-core::memory` 仍只保留最小兼容层，没有回流增强模型。
- 本轮只做 policy 接线和消费闭环，不引入 SQLite / FTS / embedding / 独立 `RecallEngine` backend。
