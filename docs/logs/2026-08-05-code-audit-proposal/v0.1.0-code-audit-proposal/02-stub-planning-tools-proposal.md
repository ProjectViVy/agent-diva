# 提案 02：清理 Agent 端的 Built-in Tools 存根、未注册工具与错置代码

## 1. 残留代码现状与定位

经过对 `agent-diva-tools` 和 `agent-diva-agent` 工具装配链路的审查，发现以下 4 处问题：

### 1.1 未注册的 `MessageTool`
- **文件路径**: [`agent-diva-tools/src/message.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-tools/src/message.rs#L21-L205)
- **残留原因**: 实现了 `Tool` Trait，但在 `agent-diva-agent/src/tool_assembly.rs` 中**没有任何注册与实例化逻辑**，Agent 无法感知或调用该工具。

### 1.2 4 个旧版 Planning Built-in Tools
- **文件路径**: [`agent-diva-tools/src/planning/mod.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-tools/src/planning/mod.rs#L184-L420)
- **残留原因**: 包含了早期架构的 `TodoShowTool`, `TodoWriteTool`, `PlanCreateTool`, `PlanSubmitTool`。生产环境已切换为 `ExecutionTodoShowTool` / `UpdatePlanTool`，这 4 个旧工具未注册且未导出。

### 1.3 Agent 端存根工具 `PlanApproveTool`
- **文件路径**: [`agent-diva-agent/src/planning/tools.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/planning/tools.rs#L42-L85)
- **残留原因**: 仅用于在 Agent 尝试审批时返回 "Unavailable" 错误提示。由于审批早已移至 Manager 控制面，此 Stub Tool 浪费 Token 且无实际功能。

### 1.4 ASCII Logo 打印代码误置在 Tools Crate
- **文件路径**: [`agent-diva-tools/src/wtf.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-tools/src/wtf.rs#L1-L15)
- **残留原因**: 仅包含控制台 Logo 打印函数，属于 CLI 控制台 UI 展示代码，仅被 `agent-diva-cli` 调用，不属于 Tool 模块。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [DELETE & MOVE]
1. 移除 `PlanApproveTool` 存根工具及 `tool_assembly.rs` 中的相关注册。
2. 彻底清理 `agent-diva-tools/src/planning/mod.rs` 中未调用的 4 个旧版 Tool 结构体。
3. 若 `MessageTool` 已被直接事件推送替代，删除 `message.rs`；若需保留则补齐注册。
4. 将 `wtf.rs` 中的 ASCII Logo 函数迁移至 `agent-diva-cli` 或 `agent-diva-core` 中。

---

## 3. 收益与风险评估
- **预期收益**：减少 LLM Tool Calling 上下文开销；保持 `agent-diva-tools` 模块职责纯粹。
- **风险分析**：无风险。底层审批和事件已有独立架构保证。
