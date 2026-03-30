# 皮层关（OFF）时的过程事件行为

**与 Story 1.4 对齐：** 《简化模式语义》正式文档计划在 **`CORTEX_OFF_SIMPLIFIED_MODE.md`**（或 Epic 1.4 登记的等价路径）中维护；本文档说明 **过程事件（FR2 发射侧）** 在皮层关时的 **实现行为**，避免与 FR3 冲突。

## 行为（当前实现）

当 `CortexRuntime::snapshot().enabled == false` 时：

- `ProcessEventPipeline::try_emit` **直接返回**，不向缓冲或下游发送任何 v0 过程事件。
- `flush_pending` 在皮层关时 **不向下游投递**（缓冲在关期间本应为空）。

## 订阅方说明

- GUI / 网关适配层在皮层关时应 **不依赖** `swarm_phase_changed` / `tool_call_started` / `tool_call_finished` 作为唯一进度信号；可依赖既有 **最终响应**、**错误事件** 或 **1.4 文档** 中登记的替代信号。

## 交叉引用

- Story **1.4** 无头测试与简化语义：`1-4-cortex-off-headless-tests` 实现工件。  
- 过程事件白名单与载荷：**[`process-events-v0.md`](./process-events-v0.md)**。  
- 皮层真相源与 Tauri 命令：**[`docs/swarm-cortex-contract-v0.md`](../../../docs/swarm-cortex-contract-v0.md)**（仓库根 `docs/`）。
