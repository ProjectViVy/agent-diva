# 提案 01：移除 Legacy Memory 兼容层与双轨记忆分支 (Memory Clean-Break)

## 1. 残留代码现状分析

### 1.1 涉及文件与位置
- [`agent-diva-agent/src/memory_boundary.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/memory_boundary.rs#L12-L200)
- [`agent-diva-core/src/config/schema.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-core/src/config/schema.rs#L290-L330)

### 1.2 背景与问题点
项目架构规定：嵌入式 Laputa (SQLite + FTS5) 是唯一的生产级 Memory 权威存储，旧版 Markdown/File-based Memory 仅作为离线导入源。

然而在当前代码中：
1. `memory_boundary.rs` 的 `memory_provider_for_mode` 中仍然包含 `MemoryAuthorityMode::Legacy` 与 `MemoryAuthorityMode::Shadow` 路径。
2. 即使在 Laputa 初始化成功时，`memory_boundary.rs` 内部的数据结构中仍然保留了 `legacy: Arc<MemoryManager>` / `Arc<dyn MemoryProvider>` 字段。
3. `config/schema.rs` 中定义了 `legacy_memory_config()` 回退函数及配置字段。

这种设计导致：
- 每次 Memory Prefetch / Recall 时仍需创建旧版 `MemoryManager` 实例；
- 代码逻辑中包含毫无必要的 Shadow 比对与 Legacy 降级分支，增加了多余的复杂度和内存开销。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [MODIFY & DELETE]

1. **`agent-diva-agent/src/memory_boundary.rs`**：
   - 完全移除 `MemoryAuthorityMode::Legacy` 与 `MemoryAuthorityMode::Shadow` 的条件分支。
   - 移除 `memory_provider_for_mode` 中对 `MemoryManager::new(workspace)` 的引用。
   - 移除 `LaputaRecallService` 中包裹的 `legacy` 成员。
   - 如果 Laputa 打开失败或损坏，统一直接返回 `DegradedMemoryProvider`，而不是降级到 Legacy 文件记忆。

2. **`agent-diva-core/src/config/schema.rs`**：
   - 标记 `MemoryAuthorityMode::Legacy` 为废弃或直接清理，统一默认模式为 `Laputa`。
   - 移除 `legacy_memory_config` 辅助函数。

---

## 3. 收益与风险评估

- **预期收益**：精简约 250 行 Rust 代码；避免运行时初始化未使用的旧版 MemoryManager；巩固 Laputa 单一权威保障。
- **风险分析**：极低。`just laputa-clean-break-check` 已经禁止旧逻辑进入生产环境。
- **验证方法**：运行 `just test` 与 `just laputa-clean-break-check`。
