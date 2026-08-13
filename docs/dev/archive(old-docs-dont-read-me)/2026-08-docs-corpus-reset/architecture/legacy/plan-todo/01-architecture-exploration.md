# Plan 与 TODO 架构设计探索

## 目标与结论

目标是把 Diva 的复杂任务流程收敛为“只读调研 → 完整计划 → 人工批准 → 可写执行 → 验证收尾”。计划生命周期、工具能力和 TODO 工作项是三个独立维度：不能再用一个 `plan` 请求标志同时表达它们。

## 参考锚定

| 参考 | 可复用原则 | Diva 适配 |
| --- | --- | --- |
| Claude Code | `EnterPlanMode`、`ExitPlanMode`、`TodoWrite` 是独立工具，[工具目录](../../../.workspace/claude-code/packages/builtin-tools/src/index.ts:15) | 拆出“开始调研”“提交计划”“批准执行”命令，不让 TODO 决定模式。 |
| oh-my-pi | Todo 是会话状态，`view` 不写入，失败批次整体回滚，[todo.ts](../../../.workspace/oh-my-pi/packages/coding-agent/src/tools/todo.ts:624) | TODO 使用版本化、原子更新；不写工作区文件。 |
| oh-my-pi | 工具声明 `read/write/exec`，未知工具默认 `exec`，[approval-mode.md](../../../.workspace/oh-my-pi/docs/approval-mode.md:3) | 用能力矩阵硬拦截，而非仅依赖提示词。 |
| OpenHarness | `/plan on` 必须刷新系统提示且禁止 mutating tools，[test_runtime_plan_mode.py](../../../.workspace/OpenHarness/tests/test_ui/test_runtime_plan_mode.py:18) | 为每个阶段切换建立端到端不变量测试。 |
| openakita | `create_plan_file` 产生可审阅计划，`exit_plan_mode` 触发审批 UI，[plan.py](../../../.workspace/openakita/src/openakita/tools/definitions/plan.py:157) | 计划作为受控规划记录；不能把该例的“计划期可 edit_file”照搬到 Diva。 |

## 当前 Diva 映射与缺口

已有 `Explore → Plan → AwaitingApproval → Execute → Verify` 生命周期，[model.rs](agent-diva-core/src/planning/model.rs:17)；`Plan`、步骤、TODO 已有持久模型，[model.rs](agent-diva-core/src/planning/model.rs:131)。但 `loop_turn` 同时从消息 `plan_mode` 与 `AwaitingApproval` 推导守卫，[loop_turn.rs](agent-diva-agent/src/agent_loop/loop_turn.rs:297)，并在守卫内允许 `todo_write`，[loop_turn.rs](agent-diva-agent/src/agent_loop/loop_turn.rs:49)。这会混淆“计划草稿”与“执行清单”。

## 目标结构

```text
User intent
  -> PlanSession (lifecycle: Explore / Drafting / AwaitingApproval / Executing / Verifying / terminal)
  -> CapabilityPolicy (Inspection / PlanningRecord / WorkItem / WorkspaceWrite / Exec / External)
  -> PlanDocument + PlanStep[]
  -> optional ExecutionTodoList (only after approval)
  -> GUI projection + audit events
```

`PlanDocument` 是审批对象；`ExecutionTodoList` 是执行投影。计划可包含候选工作项，但批准前不得把它们当作进行中的 TODO 或强制创建。
