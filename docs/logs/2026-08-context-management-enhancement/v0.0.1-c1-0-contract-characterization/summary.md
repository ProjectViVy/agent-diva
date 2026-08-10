# C1-0 上下文契约与刻画摘要

本切片建立 C1 实施前的 provider-neutral 上下文契约，并用测试冻结现有生产行为，
不改变本轮 provider 请求的消息布局或工具定义顺序。

## 变更

- 新增 `context_assembly` 模块：
  - `SectionStability`：Stable / SessionStable / TurnVolatile。
  - `ContextSection`：定义稳定前缀到当前用户消息的逻辑分区。
  - `CONTEXT_SECTION_ORDER`：冻结 C1 目标逻辑顺序。
  - `PromptSection`：承载 section、稳定性、正文和 cache-break reason 的最小骨架。
- 新增契约测试：稳定前缀必须连续位于 volatile 区之前；WM、Recall、时间、Plan、
  当前用户不得被分类为 cache prefix。
- 新增 characterization tests：冻结当前 time/session 元数据、Plan prompt、工具定义
  集合等现状；既有 Wave 3 测试继续冻结 WM → Recall 顺序。

## 影响

仅新增类型和测试；生产上下文仍由现有 `ContextBuilder`、`PreparedTurnContext` 和
`ToolRegistry` 路径生成。C1 后续切片才会迁移序列化行为。
