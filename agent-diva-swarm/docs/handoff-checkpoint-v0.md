# Handoff 检查点 v0（Story 5.3）

**范围：** FullSwarm **序曲链**（`run_swarm_deliberation_prelude`）在 **可恢复失败**（如模型/API 错误）或等价中止时，记录 **turn 内** 最后一个 **成功** LLM 步的快照。与 Epic 1 边界一致：**进程内 DTO**，不写入持久化会话；**不**进入 `ProcessEventV0` 白名单 v0（避免无 ADR 扩展 wire）。

## 查询方式

- **日志：** `tracing` 目标 `agent_diva_agent::prelude`，级别 `WARN`，结构化字段 **`checkpoint_json`**（`SwarmHandoffCheckpointV0` 的 JSON 字符串）与 **`error`**。
- **开发者：** 订阅该目标或抓取进程日志即可解析 JSON；暂无独立 HTTP API。

## 用户取消（v0 边界）

检查点 JSON 与 **`PreludeRunError`** 出现在 **序曲内某一 `chat` 报错返回** 的错误分支上（见 `agent-diva-agent` `loop_turn.rs`）。

**会话取消**在 **`process_inbound_message_inner`** 中于 **序曲 `await` 结束之后**、进入主 **`agent_turn`** 循环才轮询 `is_session_cancelled`。因此在某一序曲 **`provider.chat` 尚未完成** 的窗口内，用户点取消 **不属于 v0 书面保证范围**：**不承诺**此时必定出现带 **`checkpoint_json`** 的序曲专用 `warn!`（是否出现取决于取消如何打断 `chat` 与上层任务）。若产品要求序曲内可取消且仍需可查询检查点，需单独故事（例如在序曲步间协作取消或拆解 `chat`）。

## 载荷字段（`SwarmHandoffCheckpointV0`，JSON **camelCase**）

| 字段 | 类型 | 说明 |
|------|------|------|
| `schemaVersion` | `u32` | 固定 **`0`**。演进 bump 或经 ADR。 |
| `roleId` | `string` | 成功角色的 `phase_id`（与 `swarm-prelude` 配置一致）。 |
| `preludeRoundIndex` | `u32` | 序曲链 **0-based** 成功步索引（第一步成功为 `0`）。 |
| `summaryPreview` | `string` | 模型输出经 [`sanitize_tool_summary_for_process_event`](../src/process_events.rs) 消毒后截断（默认最多 **256** 标量），**非**完整原文。 |
| `contentFingerprintHex` | `string` | 对消毒全文（上限 **4096** 标量）做 **FNV-1a 64** 的十六进制指纹（**16** 个小写 hex 字符），供对账。 |

## NFR

- **NFR-S2：** 预览与指纹输入均走同一套消毒（控制字符压平、空白折叠、截断）。
- **NFR-R2：** 检查点 **不** 默认写入用户 transcript；仅日志与（未来可扩展的）内部 API。

## MVP 行为（二选一，**已封冻**）

**仅报告（report-only）：** 序曲在某一角色 `chat` 失败时，主 turn **不** 自动在同 turn 内重试序曲；主 ReAct 在 **无** 序曲注入摘要的情况下继续（与实现一致）。禁止未文档化的第三种行为（例如静默丢弃已成功的相位/遥测而不留检查点日志）。

跨 turn 自动恢复不在 v0 范围，仅预留 `schemaVersion` 与字段扩展空间。

## 与 `ProcessEventV0` 的关系

- v0 白名单 **不** 新增事件名；检查点 **不** 经 `ProcessEventPipeline::try_emit` 下发 GUI。
- 序曲成功步仍仅产生既有的 `swarm_phase_changed`（在对应 `chat` 成功之后），与 Story 5.1/5.2 一致。

## 代码入口

- DTO：`agent-diva-swarm/src/handoff_checkpoint.rs` — `SwarmHandoffCheckpointV0`
- 序曲：`agent-diva-agent/src/agent_loop/loop_turn.rs` — `run_swarm_deliberation_prelude`、`PreludeRunError`
