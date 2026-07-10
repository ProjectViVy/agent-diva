# 实现方案

## 总体方案

保留现有 `Plan`/`PlanStep`/`TodoList` 存储模型，新增显式的 `PlanModeState` 与 `ToolCapability`。运行时只从状态机派生能力，GUI 只投影状态，模型提示词只是辅助约束。批准是唯一将计划草稿物化为执行上下文的边界。

## 变更清单

| 位置 | 变更 | 影响 |
| --- | --- | --- |
| [model.rs](agent-diva-core/src/planning/model.rs:17) | 增加 `PlanModeState`、`PlanApproval`、候选 TODO 标记/来源 | 核心模型 |
| `agent-diva-core/src/planning/policy.rs`（新增） | 生命周期到能力矩阵的唯一实现 | 新模块 |
| [loop_turn.rs](agent-diva-agent/src/agent_loop/loop_turn.rs:49) | 替换名称白名单，统一调用 policy | 每轮工具装配 |
| [loop_runtime_control.rs](agent-diva-agent/src/agent_loop/loop_runtime_control.rs:340) | 批准时冻结计划版本、可选生成 TODO | 审批边界 |
| [planning/mod.rs](agent-diva-tools/src/planning/mod.rs:49) | 将全量替换 TODO 改为带 revision 的补丁命令 | TODO 工具 |
| [App.vue](agent-diva-gui/src/App.vue:34) | 使用后端状态渲染阶段与能力，而非 `ExecMode` 推测 | UI |

## 实施顺序

1. 在 core 定义状态、转移表、能力策略，并为非法转移返回领域错误。
2. 在 tools 引入 `plan_submit`；`todo_create_from_plan` 只允许批准后的执行态调用。
3. 让 agent loop 在工具注册、调用前、调用后均查询策略（纵深防御）。
4. 在批准处理处比较 `plan_revision`，记录批准人、时间、批准版本；需要清单时原子生成 TODO。
5. 让 GUI 从 `PlanRuntimeState` 呈现阶段、计划内容、能力说明和审批动作。

## 接口草案

```rust
enum PlanModeState { Exploring, Drafting, AwaitingApproval, Executing, Verifying, Closed }
enum ToolCapability { Inspect, PlanningRecord, WorkItem, WorkspaceWrite, Execute, External }

fn allows(state: PlanModeState, capability: ToolCapability) -> bool;
fn approve(plan_id: &PlanId, expected_revision: i32, todo_policy: TodoPolicy) -> Result<ApprovalReceipt, PlanningError>;
```

无 `unwrap`/`expect`；所有落库操作返回 `Result`，并通过既有 store 的事务/比较更新保证原子性。
