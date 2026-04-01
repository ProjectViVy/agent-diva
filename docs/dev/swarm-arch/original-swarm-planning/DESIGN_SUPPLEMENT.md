# Agent-Diva — 设计补充（v0）

本文档承接架构审查结论，补充 **`ARCHITECTURE_DESIGN.md` 尚未展开** 的实现向约定：可观测性、内外 transcript、Meta 对齐、合成策略规格、安全与测试等。

**前置阅读：** [研究总览](./AGENT_DIVA_SWARM_RESEARCH.md) · [能力深挖](./CAPABILITY_ARCHITECTURE_DEEP_DIVE.md) · [**Rust 架构设计**](./ARCHITECTURE_DESIGN.md)

**日期：** 2026-03-30

---

## 1. 可观测性与调试

### 1.1 推荐 Span / 日志属性（最低集）

每条 **对外** 或 **对内 LLM 调用**、**工具执行**、**合成** 建议携带（名称可按 OTEL 惯例调整）：

| 属性 | 说明 |
|------|------|
| `agent_diva.session_id` | `PersonSession` 标识 |
| `agent_diva.swarm_member_id` | 当前行动成员（无则空） |
| `agent_diva.capability_id` | 当前能力绑定 |
| `agent_diva.lease_holder` | `SteeringLease` 持有人 |
| `agent_diva.swarm_tick` / `iteration` | 内层循环序号 |
| `agent_diva.synthesis_policy` | 使用的合成策略名 |
| `agent_diva.synthesis_decision` | 摘要：合并了哪些输入、是否 HITL |

工具调用另加：`tool.name`、是否 `is_dangerous`、enforcement 是否 drop。

### 1.2 「控制室录像」（开发态）

- **默认**：用户只看到经 `PersonOutbox` 流出的内容。  
- **可选 dev flag**（如 `AGENT_DIVA_DEBUG_COUNCIL=1`）：将 **黑板增量、邮箱摘要、lease 变更、合成输入哈希** 写入 **单独调试流**（文件或结构化日志），**不得**默认混进用户 transcript。  
- 用途：对齐「头脑特攻队」式排障，而不破坏「对外一个人」的产品叙事。

---

## 2. 用户可见 transcript 与内部 trace

### 2.1 分轨

| 轨道 | 内容 | 持久化建议（v0） |
|------|------|------------------|
| **User-visible** | 经 lease 持有人路径聚合后的、对用户展示的消息 | 内存 + 可选会话文件；与产品 CLI/API 一致 |
| **Internal trace** | 各成员 raw 片段、工具 I/O 引用、黑板 key 版本 | 内存；Phase 2 可选仅 debug 落盘 |
| **Audit / compliance**（若需要） | 不可变追加、谁批准了何种工具 | 独立存储策略，与 UX transcript 分离 |

### 2.2 PreCompact / 压缩时的「保留集」

与 **双环记忆**（主文档已提）对齐为可操作规则：

- **必保留**：用户消息、已提交的对外回复、`PersonSession` 级目标摘要、**已提交 artifact 的 URI**（含 file-as-memory 路径）。  
- **可丢**：探测性中间句、未合成草稿、已过收敛轮的邮箱副本（若已提炼到黑板）。  
- **PreCompact hook**（meta）仅允许 **缩小内部 trace**，对用户可见轨的删改需 **显式策略**（或禁止，仅摘要附加）。

### 2.3 合规与钩子

若 Meta 或运维钩子 **改写对外文本**，须在审计轨记录 **before/after 哈希 + 策略 id**，避免与「单一 Person 叙事」冲突时无法追责。

---

## 3. Meta 事件与 oh-my-claudecode（OMC）对齐

便于实现时 **不漏事件**；**v0** 列可裁剪范围。

| OMC / Claude 生态事件 | `agent-diva-meta` 建议名 | v0 |
|------------------------|---------------------------|-----|
| `UserPromptSubmit` | `UserPromptSubmit` | ✅ |
| `SessionStart` | `SessionStart` | ✅ |
| `SessionEnd` | `SessionEnd` | ✅ |
| `PreToolUse` | `PreToolUse` | ✅ |
| `PostToolUse` | `PostToolUse` | ✅ |
| `PostToolUseFailure` | `PostToolUseFailure` | 建议 ✅（与 PostToolUse 对称，利于重试/熔断） |
| `PermissionRequest` | `PermissionRequest` | Phase 2（或等价合并到 PreToolUse + 策略位） |
| `Stop` | `Stop` | ✅ |
| `SubagentStop` | `SubagentStop` | Phase 2（外群/IDE 子任务时） |
| `SubagentStart` | `SubagentStart` | Phase 2 |
| `PreCompact` | `PreCompact` | ✅ |
| `Notification` | `Notification` | Phase 2（推送/IDE） |

**说明：** OMC 用 shell/node 链；Agent-Diva 要求 **进程内 `MetaBus`**，语义对齐即可，实现不抄脚本模型。

---

## 4. `SynthesisPolicy` 可测试规格

### 4.1 输入（建议结构化类型，放在 `domain` 或 `protocol`）

- `proposals: Vec<MemberProposal>`：成员 id、capability_id、**结构化载荷**（JSON 或强类型）、可选 `confidence: f32`。  
- `blackboard_snapshot: HashMap<Topic, Vec<Entry>>`（只读视图）。  
- `lease_before: SteeringLease`（合成后是否移交由策略输出）。  
- `user_turn_context: UserMessageRef`（本轮用户意图锚点）。

### 4.2 输出

- `merged_artifact`：进黑板或 file-as-memory 的**单一**结果。  
- `user_visible_delta`：可选；若为空则本轮对外可仅 ACK。  
- `lease_after`：是否 handoff。  
- `hitl_request`：可选；若 set，runtime **暂停**自动循环直至用户/操作员响应（对齐 CrewAI HITL、Shannon 审批思想）。

### 4.3 两种实现路径（与 `ARCHITECTURE_DESIGN` §8 风险表一致）

1. **确定性**：规则合并、按 topic 覆盖、置信度阈值（类官方 `code-review` ≥80 过滤思想）。  
2. **LLM Chair**：小模型、固定输出 schema；**须**有 golden 测试与回归集。

### 4.4 测试契约

- **单元**：纯函数输入 `MemberProposal` 列表 → 期望 `merged_artifact` 与 `lease_after`。  
- **集成**：mock `LlmClient`，固定返回 `SwarmAction` JSON，跑完整 tick → `RuntimeEffect` 序列。

---

## 5. 跨成员安全与信任

- **黑板 / 邮箱**：默认视为 **不可信用户输入**；注入模型前做 **长度上限、topic 白名单、可选 schema 校验**。  
- **成员 A 的工具结果** 被 **成员 B** 消费时：由 **Capability** 声明 **`read_topics` / `trusted_sources`**；runtime 在组装 prompt 时强制执行。  
- **高危工具**：沿用 Shannon 思想 — `is_dangerous` + PreToolUse **默认拒绝**，直至 manifest 与 lease 策略双重允许。

---

## 6. File-as-memory（内群）

借鉴 Shannon swarm：**长工具结果、报告正文写入工作区文件**，成员在 `done` 或等价动作中 **只返回短摘要 + 路径**。

- **约定**：路径落在会话隔离目录（如 `.agent-diva/sessions/{id}/artifacts/`），禁止裸写仓库根。  
- **黑板** 存引用（path + 内容 hash + 作者 member_id），避免把全文塞进 prompt。  
- **合成阶段** Chair 可读文件头或摘要行决定是否拉全量。

---

## 7. Handoff 上下文继承与裁剪

对齐 OpenAI Agents SDK 思路（`input_filter`、`nest_handoff_history`），避免 **lease 转让** 时 token 爆炸或 **上下文污染**：

- **策略表**：按 `CapabilityId` 配置「继承上轮消息条数 / 仅 system+工具结果 / 清空工具史」。  
- **多跳 handoff**：文档化 **last-wins**（与历史 Swarm 一致）与 **每跳是否重新裁剪**。  
- **ADR**：见 [`ARCHITECTURE_DESIGN.md`](./ARCHITECTURE_DESIGN.md) **§6 ADR-D（草案）**；本节为操作细则。

---

## 8. 测试与协议契约

- **`agent-diva-protocol`**：`SwarmAction` 等 JSON **golden files**（有效例 + 应拒绝的无效例）。  
- **`LlmClient` mock**：返回预定 tool_calls / 文本，驱动 `swarm` **run_loop** 无网络。  
- **Runtime**：集成测试断言 **无任何用户可见写** 在未持有 lease 时发生（守卫 `PersonOutbox`）。  
- **CI**：`cargo tree` 或脚本断言 **`agent-diva-swarm` 不依赖 `agent-diva-meta`**（与 ADR-A 一致）。

---

## 9. 工作区内其他仓库（范围说明）

- **`zeroclaw` 等**：若与 **桌面壳 / Tauri / 本地 UI** 集成，单独文档化 **IPC 边界**（何者持有 `PersonSession`、何者只渲染流），避免与 `agent-diva-runtime` 职责重叠。  
- **未纳入参考栈的目录**：在 `AGENT_DIVA_SWARM_RESEARCH.md` 中可保持「非 v0 范围」，此处不展开实现。

---

## 10. 修订记录

| 日期 | 说明 |
|------|------|
| 2026-03-30 | 初版：审查备忘落地（可观测性、transcript、Meta 对齐、合成规格、安全、file-as-memory、handoff 裁剪、测试、仓库范围） |
