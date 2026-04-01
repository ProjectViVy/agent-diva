# Subagent → Swarm 迁移清单（自动执行产出）

**生成日期：** 2026-03-30  
**执行方式：** 并行子代理调研（`agent-diva` 树内实现 + 仓库根 `.cursor/agents` + `agent-diva-swarm/docs`）与代码检索合并。  
**术语对齐：** Person 对外单一叙事；对内 swarm；**Capability** = 工具/提示/模型档/门禁契约；**Voice** = 审议声部；**handoff** / **as_tool** / **并行 gather** 见 `agent-diva-swarm/docs/CAPABILITY_ARCHITECTURE_DEEP_DIVE.md`。

---

## 1. 三层「子代理」不要混为一谈

| 层级 | 位置 | 现状 | Swarm 改造语义 |
|------|------|------|----------------|
| **A. 运行时后台子代理** | `agent-diva-agent/src/subagent.rs` + `agent-diva-tools/src/spawn.rs` | 单一 `SubagentManager`：`spawn` 工具启任务，共享 `LLMProvider`，独立上下文；工具集 = list/read/write、`ExecTool`、Web；无嵌套 spawn；结果经 bus 回主循环 | **对内 swarm 工作者**：应对齐 **CapabilityBinding**（工具子集已有雏形）、**预算/超时**（已有 `exec_timeout`）、**收敛**（结果经主 agent「自然概括」— 应对齐 **Chair / synthesis** 与 PersonOutbox，而非 ad-hoc 文案） |
| **B. Cursor 具名 subagent（研发用）** | `d:\newspace\.cursor\agents\*.md`（5 个） | 研究/综述类提示，服务文档与架构迭代 | **外群 IDE worker**，不进入 Agent-Diva 运行时；与 **内群 Voice** 在工程边界上分离（对齐 OMC `SubagentStart/Stop` 类区分） |
| **C. BMAD / 技能包内嵌 agent** | 如 `bmad-distillator/agents/*.md`、`bmad-product-brief/agents/*.md` 等 | Cursor 工作流子代理 | 同 **B**，产品 **Person** 无感知；若将来 GUI 复刻能力，再映射为 **Capability 包** 而非 1:1 抄提示词 |

---

## 2. A 层（Rust）— 唯一「必须改代码」的 subagent

| 组件 | 路径 | 当前行为 | 建议 swarm 映射 |
|------|------|----------|-----------------|
| `SubagentManager` | `agent-diva-agent/src/subagent.rs` | 后台 Tokio 任务 + UUID；`build_subagent_prompt` 注入 SOUL/IDENTITY/USER | **SwarmMember** 或 **并行分支执行器**：声明 `capability_id` / 工具白名单（已有固定集合，可升格为注册表项） |
| `SpawnTool` | `agent-diva-tools/src/spawn.rs` | 主 agent 调用 `spawn` 启后台任务 | **request_help / 动态增员** 的 v0：`spawn` 保留为 UX，内核逐步改为 swarm 调度同一 **SteeringLease** 下多成员 |
| 工具注册 | `subagent.rs`（`execute_subagent_task` 附近） | `ListDirTool, ReadFileTool, WriteFileTool, ExecTool, WebFetchTool, WebSearchTool` | 每项应对齐 **ToolCapability** 元数据（危险度、权限、限流）— 与 Shannon 调研一致 |
| 回传 | `announce` → `InboundMessage` | 要求主模型「简要概括、不提 subagent」 | 对齐 **单一对外出口**：中间态进黑板/议会记录，**仅合成结果** 进 Person 流 |

---

## 3. B 层 — `.cursor/agents` 具名列表（研发资产）

| 文件 | 角色（简述） | Swarm 文档中的类比 |
|------|--------------|-------------------|
| `agent-diva-synthesizer.md` | 合并多源研究为 Agent-Diva 架构简报 | 人类侧 **synthesis** 流程参考，非运行时 |
| `lightweight-swarm-handoff-researcher.md` | Swarm vs Agents SDK / handoff | **handoff / steering** 设计输入 |
| `crew-flow-researcher.md` | CrewAI Flows / 状态路由 | **Voice 轮次 / DAG** 设计输入 |
| `claude-omc-hooks-researcher.md` | OMC hooks、Subagent 事件 | **Turn 间 Meta** 与 IDE subagent 边界 |
| `shannon-orchestration-researcher.md` | Shannon 工具门禁、swarm 协议 | **Capability / enforcement** 设计输入 |

**改造动作：** 无需「改代码」；在 `agent-diva-swarm/docs` 中保持索引即可。若重命名角色，只影响 Cursor 调用名，与 Rust **无关**。

---

## 4. C 层 — 技能 `agents/openai.yaml`（产品元数据）

| 路径 | 用途 |
|------|------|
| `agent-diva/.skills/agent-diva-rust-dev/agents/openai.yaml` | IDE 中技能展示名 |
| `agent-diva/.skills/agent-diva-gui-pm-ui/agents/openai.yaml` | 同上 |
| `agent-diva/.skills/agent-diva-agent-design/agents/openai.yaml` | 同上 |

**改造动作：** 与 swarm **无强制映射**；若品牌上统一「Person / Capability」用语，可改 `short_description` 文案。

---

## 5. 你之前四步里的「自动执行」落点

| 步骤 | 本次已执行 | 仍需工程/产品接手 |
|------|------------|-------------------|
| **1. 清单化** | 本文档 + 并行子代理路径核实 | 评审后冻结 v0 **Capability** 表（从现有 subagent 工具集扩展） |
| **2. 单路径 e2e** | 未跑自动化测试 | 选一条：`spawn` → 子循环完成 → 主循环回复；接 **trace** 验证仅一条用户可见叙事 |
| **3. 门禁** | 未改代码 | 对 `ExecTool` / `WriteFileTool` 等挂 **Capability** 危险标记与 enforcement（与 `architecture.md` / ADR 对齐） |
| **4. 体验 polish** | 未做 | GUI `swarm/` 组件与 PRD 中过程反馈一致（见 `ux-design-specification.md`） |

---

## 6. 与现有规划的挂钩

- **Epics / ADR-A：** `epics.md` 已约束 swarm crate 与 meta 边界；本清单 **A 层** 是 `agent-diva-agent` 与未来将引入的 `agent-diva-swarm` **接缝**。  
- **深度设计：** `agent-diva-swarm/docs/ARCHITECTURE_DESIGN.md`（`SteeringLease`、`RuntimeEffect`）为 A 层改造的规范来源。

---

## 7. 建议的下一可提交故事（供 backlog）

1. **SWARM-MIG-01：** 将 `SubagentManager` 工具列表抽为 **CapabilityRegistry** 可引用条目（行为不变，仅数据驱动）。  
2. **SWARM-MIG-02：** 子任务结果路径显式标注为 **internal** vs **person_visible**（为 PersonOutbox 铺路）。  
3. **SWARM-MIG-03：** 文档：在 `agent-diva/docs/dev/architecture.md` 增加「三层 subagent」示意图，链到本文档。

---

*本文件由自动化并行调研生成；若目录或文件有增减，以仓库为准更新 §3–§4。*
