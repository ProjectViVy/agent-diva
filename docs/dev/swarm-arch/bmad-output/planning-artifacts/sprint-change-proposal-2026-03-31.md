# Sprint Change Proposal — 蜂群主线与 Agent 循环脱节（Correct Course）

**项目：** newspace / agent-diva  
**日期：** 2026-03-31  
**触发：** 集成测试后发现 — **大脑皮层开启后，实际对话路径仍等同「单代理 + spawn 子代理」**，未出现蜂群架构下可预期的多角色协作与过程语义。  
**撰写对象：** Com01（配置：`communication_language` / `document_output_language` = 简体中文）

---

## 1. 问题摘要（Issue Summary）

### 1.1 现象

- GUI 上大脑皮层为 **ON**、过程事件管道已接入时，用户仍观察到 Diva 的 **行事逻辑与旧版 subagent spawn 一致**。
- **根因（代码层面，已核实）：**
  - `agent_diva_swarm` 中的 **`resolve_execution_tier`、`run_minimal_turn_headless`、收敛循环** 仅用于 **headless 单测与桩路径**，**未** 接入真实 `AgentLoop::process_inbound_message_inner`（`agent-diva-agent/src/agent_loop/loop_turn.rs`）。
  - 生产路径始终是：**单会话 ReAct 循环 + `SpawnTool` → `SubagentManager`**；皮层状态主要门控 **过程事件发射**，**不** 改变编排拓扑。
  - 因此与 PRD/架构中「皮层 ON 时可走完整多参与者编排」的 **用户预期** 出现落差；短问答（FR19 轻量类）本就不应升全图，但 **长任务或显式深度** 此前在 Agent 侧也 **没有** 对应的蜂群语义实现。

### 1.2 证据

- `loop_turn.rs` 在变更前 **无** `resolve_execution_tier` / `ExecutionTier` 分支。
- `minimal_turn.rs` 文档写明为 **headless 桩**，与 GUI 主路径无关。
- `handlers.rs` 的 `ChatRequest` 原无「显式全蜂群」字段，无法把 UI 选择传到 Agent。

---

## 2. 影响分析（Impact Analysis）

### 2.1 Epic / 故事影响

| 区域 | 影响 |
|------|------|
| Epic 1（蜂群基础设施） | 已有 **契约与测试** 有效，但 **与 Agent 主循环集成缺口** 暴露为 P0；需新增/调整故事：**「执行分层接入 AgentLoop」**、**「可观测多角色序曲 / handoff 最小实现」**。 |
| FR19 | 轻量路径不变；需保证 **显式深度** 与 **长非轻量输入** 在 Agent 侧真实进入 FullSwarm 语义。 |
| FR20 | 收敛策略须在 **真实 turn** 上可观测（终局 `swarm_run_*` 与内部阶段一致）。 |
| FR8 / FR9 | 仍保持 **单一 Person 对外叙事**；多代理仅 **系统侧与过程条**，不增加并列聊天流。 |

### 2.2 制品冲突

- **architecture.md / ADR-E：** 已描述 Light vs FullSwarm；需补充一句 **「权威路由须在 AgentLoop 入口与 swarm crate 共用同一 `resolve_execution_tier`」**（避免双实现）。
- **PRD：** 无目标冲突；需强调 **MVP 蜂群 ≠ 仅 UI 事件**，须包含 **编排分叉**。
- **UX：** 已具备 `ProcessFeedbackStrip`；需 **可选控件** 表达「本则消息显式深度编排」（与 FR19 一致）。

### 2.3 技术影响

- **API：** `POST /api/chat` 增加可选字段 `explicit_full_swarm`（JSON）。
- **Tauri：** `send_message` 增加可选参数并下传网关。
- **成本：** FullSwarm turn 在实现中包含 **额外 2 次 `chat`（非流式）** 序曲，需在设置/文档中标注（对齐 FR22 挂点后续可增强）。

### 2.4 对标参考（经典蜂群 / 编排）

| 来源 | 可借鉴点 |
|------|----------|
| **OpenAI Swarm（历史 handoff 模型）** | 多角色、**显式 handoff**、控制流在编排层而非单工具 spawn。 |
| **openai-agents-python** | **agents / handoffs**、Runner 级 **trace**、与「单活跃代理 + 交接」一致的语义。 |
| **Shannon（Kocoro-lab）** | 生产向 **DAG / 工作流编排**、预算与 **可观测阶段**；长期可对齐「图编排 + 强制执行层」。 |

当前代码变更：**不** 一次引入 Shannon 级工作流引擎；先落地 **handoff 式序曲 + 主代理综合 + 禁用 spawn**，为后续接 Shannon/Agents SDK 式图编排 **预留同一 `ExecutionTier` 入口**。

---

## 3. 推荐路径（Recommended Approach）

**选项：** **方案 1（直接调整）+ 分阶段加深**  

- **阶段 A（已完成初版实现）：**
  - AgentLoop 在存在 `ProcessEventPipeline` 时，用 **`pipe.cortex_runtime()`** 读取皮层状态，调用 **`resolve_execution_tier`**。
  - **FullSwarm：** 主循环前 **规划代理 + 批评代理** 两轮 `chat`，经 `swarm_phase_changed` 可观测；摘要注入 **系统消息**；本 turn **从工具列表移除 `spawn`**；turn 成功结束前调用 **`execute_full_swarm_convergence_loop`** 发射 **`swarm_run_finished`**（与 FR20 终局事件一致）。
  - 网关 **`ChatRequest.explicit_full_swarm`** + GUI **「本则蜂群深度编排」** 勾选（皮层开时显示）。
- **阶段 B（后续故事）：** 将序曲扩展为 **可配置角色图**、**可恢复 handoff 状态**、与 **RunTelemetry / FR22** 统一计数；评估 **嵌入式 Python Agents** 或 **独立编排进程**（视 ADR-A 边界而定）。

**不采纳：** 全面回滚 Epic 1（损失已验证契约）。  
**MVP 审查：** 不必砍 MVP，但须接受 **「蜂群 v0 = 序曲 + 单主循环」**，与愿景中的 Shannon 全图 **分阶段**。

**风险：** 中等 — 多 2 次 LLM 调用与延迟；**缓解：** 仅 FullSwarm tier、可配置开关与预算后续故事补齐。

---

## 4. 详细变更说明（对应实现 / 待办）

### 4.1 已修改文件（阶段 A）

- `agent-diva-swarm/src/process_events.rs`：`ProcessEventPipeline::cortex_runtime()`。
- `agent-diva-agent/src/agent_loop/loop_turn.rs`：执行分层、序曲、`spawn` 过滤、终局收敛事件。
- `agent-diva-manager/src/handlers.rs`：`ChatRequest.explicit_full_swarm` → `metadata`。
- `agent-diva-gui/src-tauri/src/commands.rs`：`send_message` 下传字段。
- `agent-diva-gui`：`ChatView.vue`、`NormalMode.vue`、`App.vue`、i18n `zh.ts` / `en.ts`。

### 4.2 建议后续文档 / 故事（阶段 B）

- 更新 `architecture.md` ADR-E：**AgentLoop 与 `resolve_execution_tier` 单一入口**。
- 新增故事：**可配置蜂群角色 / 最大序曲轮次**、**与 Shannon 编排适配层 SPI**。

---

## 5. 实施交接（Implementation Handoff）

| 分类 | **Major**（编排语义与产品预期错位，已部分在代码层修复，仍需架构文档与后续故事） |
|------|-----------------------------------------------------------------------------|
| **接收方** | 开发：继续阶段 B；PO/SM：补故事与验收标准；架构：对齐 Shannon/Agents 长期图。 |
| **成功标准** | 皮层 ON +（长非轻量输入 **或** 显式勾选）时，过程条出现 **蜂群阶段**（规划/批评/合并），且 **本 turn 不出现 spawn 工具调用**；终局出现 **`swarm_run_finished`**；皮层 OFF 行为仍符合 FR3。 |

---

## 6. Checklist 执行记录（节选）

| 节 | 项 | 状态 |
|----|----|------|
| 1 | 触发故事 | 集成测试 / 用户验收 → **Action-needed：补登记故事 ID** |
| 1 | 问题类型 | **实现与需求脱节**（非单纯误解 PRD） |
| 2 | Epic 1 | **需增补集成故事**，非推翻 epic |
| 3 | PRD/架构 | **需小补**（单一入口、对标说明） |
| 4 | 路径 | **选项 1 + 分阶段** |
| 6 | 用户批准 | **待你回复「批准 / 修订」**；若批准，可将 `sprint-status.yaml` 中相关故事标为 in-progress / review |

---

## 7. 使用说明（给 Com01 测通）

1. **大脑皮层保持开启**；在聊天顶栏勾选 **「本则蜂群深度编排」**（短消息也可强制 FullSwarm，符合 FR19 显式深度）。  
2. 或不勾选，但发送 **超过 256 标量字符** 且 **非** `/`、非 `bmad-` 的轻量样式，则在皮层 ON 时也会进入 FullSwarm。  
3. 观察 **过程反馈条**：应出现 `swarm_peer_planner` / `swarm_peer_critic` / `swarm_peer_merge` 等阶段文案，然后主回答流式输出；**不应** 再走 `spawn` 子代理工具链。

---

**Correct Course 流程说明：** 若你 **批准** 本提案，下一阶段以阶段 B（可配置角色、预算与文档同步）为主；若 **修订**，请直接说明希望优先 Shannon 对接还是 Agents SDK 式 handoff 深度对齐。
