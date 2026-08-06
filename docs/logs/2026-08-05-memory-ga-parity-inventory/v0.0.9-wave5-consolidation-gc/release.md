# Release — GA-MEM-PARITY Wave 5

## 发布方式

无独立发布。Wave 5 为纯内部接线 + 测试 + 文档变更，不影响用户可见 CLI/GUI
行为。变更随 `agent-diva-pro` 分支常规发布。

## 向后兼容性

| 变更 | 影响 |
|------|------|
| `TypedLaputaMemoryProvider::open()` 新增 `active_session_ids` 入参 | `Option` 默认 `None` = 不 GC；现有调用点无需改 |
| `BoundedReflectionInput.superseded_memory_digests` | `#[serde(default)]`；旧 JSON 无此字段正常反序列化 |
| `save_memory` tool schema v3 | 仅 consolidation 内部使用；用户工具面不变 |
| `CandidateRejectionCode::Superseded` | 新变体；AutoDream 输出语义不变（rejected 数组 reason 更精确） |
| `distill_guard` thread-local flag | 新增模块；不影响现有行为 |

## 监控指标变化

- AutoDream worker 日志中 `Duplicate` 计数可能下降、`Superseded` 上升
  （更精确的拒绝原因）——非回归。
- Consolidation 日志新增 `"Consolidation itemized: N applied, N proposed, N failed"`
  和 `"consolidation fallback: non-itemized"` 文案。
