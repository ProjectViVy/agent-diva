# 提案 01：清理 Agent Core & Memory 遗留适配器、影子模式与孤立死代码

## 1. 残留代码现状与定位

经过对 `agent-diva-core` 和 `agent-diva-agent` 两个核心 Crate 的深度审计，梳理出以下 8 处主要残留点：

### 1.1 计划审批治理兼容适配层 `governance_adapter.rs`
- **文件路径**: [`agent-diva-core/src/planning/governance_adapter.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-core/src/planning/governance_adapter.rs#L1-L135)
- **残留原因**: 文件头部明确注释 `//! These functions are not wired into the production planning store.`，全仓除自身单测外 **0 处生产调用**。

### 1.2 已废弃的全局 Plan 审批控制命令
- **文件路径**: 
  - [`agent-diva-agent/src/runtime_control.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/runtime_control.rs#L52-L60) (`ApproveActivePlan`, `ReturnActivePlanToDraft`)
  - [`agent-diva-agent/src/agent_loop/loop_runtime_control.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/agent_loop/loop_runtime_control.rs#L117-L126)
- **残留原因**: Handler 中硬编码无条件返回 `"legacy global plan approval has been removed"` 错误，外部无任何发送方构造这两个变体。

### 1.3 记忆系统 Legacy / Shadow 影子比对逻辑
- **文件路径**: 
  - [`agent-diva-agent/src/memory_boundary.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/memory_boundary.rs#L75-L209) (`CutoverMemoryProvider`)
  - [`agent-diva-core/src/memory/recall.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-core/src/memory/recall.rs#L160-L169) (`RecallShadowReport`, `compare_recall_shadow`)
- **残留原因**: Laputa 嵌入式数据库已是唯一生产权威，过渡期用于对比旧 `MEMORY.md` 和 Laputa typed store 的 Shadow 影子比对器与增量统计函数已完成历史使命。

### 1.4 `SqlitePlanningStore` 内部无用 Row 结构体
- **文件路径**: [`agent-diva-core/src/planning/store.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-core/src/planning/store.rs#L1129-L1280)
- **残留原因**: `PlanRow`, `StepRow`, `TodoRow`, `EventRow` 四个结构体被标记 `#[allow(dead_code)]`，底层 SQL 查询全部直接绑定参数而未反序列化到这些 Row 类型。

### 1.5 孤立结构体 `NagTracker` & `TodoPlanner` & 安全 Stub
- **文件路径**: 
  - [`agent-diva-agent/src/planning/nag.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/planning/nag.rs#L1-L112) (`NagTracker`)
  - [`agent-diva-agent/src/planning/todo_planner.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/planning/todo_planner.rs#L1-L126) (`TodoPlanner`)
  - [`agent-diva-core/src/security/injection.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-core/src/security/injection.rs#L287-L293) (`model_based_check`)
  - [`agent-diva-core/src/security/pii.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-core/src/security/pii.rs#L131-L159) (`CustomPiiRule`)
- **残留原因**: 均属于未与实际业务流程挂钩、无调用的孤立结构体或返回固定空值的 Stub 代码。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [DELETE & REFRACTOR]
1. 删除 `governance_adapter.rs` 全文件（135 行）。
2. 从 `RuntimeControlCommand` 中移除 `ApproveActivePlan` / `ReturnActivePlanToDraft` 变体及其 Handler。
3. 清理 `memory_boundary.rs` 中的 `CutoverMemoryProvider` 及 `recall.rs` 中的影子比对代码。
4. 删除 `store.rs` 中被 `#[allow(dead_code)]` 压制的 4 个 Row 结构体。
5. 移除孤立的 `NagTracker`、`TodoPlanner` 以及 `injection.rs` / `pii.rs` 中的未接入 Stub。

---

## 3. 收益与风险评估
- **预期收益**：精简 Agent Core 模块约 800 行代码，彻底巩固 Laputa 单轨权威，消除运行时多余比对开销。
- **风险分析**：极低。`just test` 与 `just laputa-clean-break-check` 可确保安全。
