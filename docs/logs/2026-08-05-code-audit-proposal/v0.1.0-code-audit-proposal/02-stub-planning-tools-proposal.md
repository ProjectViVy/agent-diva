# 提案 02：清理 Agent 端失效的计划审批与存根工具 (Stub Tools Clean-up)

## 1. 残留代码现状分析

### 1.1 涉及文件与位置
- [`agent-diva-agent/src/planning/tools.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/planning/tools.rs#L42-L85) (`PlanApproveTool`)
- [`agent-diva-agent/src/tool_assembly.rs`](file:///C:/Users/Administrator/Desktop/morediva/agent-diva/agent-diva-agent/src/tool_assembly.rs#L205-L215)

### 1.2 背景与问题点
在 HITL 审批重构（GMH-30..33）后，所有 Plan 审批与风险命令的决策权均转移到了 Manager 统一治理层（HTTP/Tauri/CLI 控件），不再允许 Agent 自行调用工具进行“自审批”。

但是在代码中：
1. `PlanApproveTool` 仍然作为 `Tool` trait 的实现保留在 `planning/tools.rs` 中。
2. 其 `execute` 方法仅硬编码返回错误：`ToolError::ExecutionFailed("Plans require an explicit revision-bound user approval through runtime control.")`。
3. `tool_assembly.rs` 在组装工具列表时仍会注入这个只用于报错的 Dummy Stub Tool。

这类 Dummy Tool 占用 LLM 的 System Prompt 上下文（Tool Definition Token），且增加了理解负担。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [DELETE]

1. **`agent-diva-agent/src/planning/tools.rs`**：
   - 彻底删除 `PlanApproveTool` 结构体及 `impl Tool for PlanApproveTool` 块（约 45 行代码）。

2. **`agent-diva-agent/src/tool_assembly.rs`**：
   - 移除 `PlanApproveTool` 的实例化与注册代码。

3. **Prompt 上下文同步**：
   - 确认 System Prompt 渲染逻辑不再提及 `plan_approve` 工具。

---

## 3. 收益与风险评估

- **预期收益**：减少 LLM Prompt 消耗；避免 Agent 尝试调用不可用工具导致的无效 Turn 浪费。
- **风险分析**：无风险。底层审批已有 Manager API 坚固保障。
- **验证方法**：运行 `cargo test -p agent-diva-agent` 确认工具装配逻辑正常。
