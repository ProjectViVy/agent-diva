# Agent 主动询问（Clarify / Ask-User）HITL 未闭环 — 研究与提案

- 日期：2026-08-05
- 类型：根因研究 + 实现前提案（本回合仅文档归档，无产品代码变更）
- 状态：研究已归档；实现待独立回合（见 TODOLIST `CLARIFY-HITL`）
- backlog：`TODOLIST.md` → **CLARIFY-HITL：ask_user 对话询问闭环**
- 迭代日志：[`docs/logs/2026-08-ask-user-hitl-research/v0.0.1-research-archive/`](../logs/2026-08-ask-user-hitl-research/v0.0.1-research-archive/)
- 相关但**不同**域：
  - Governance HITL / 审批：[`sandbox-hitl-approval-policy-proposal.md`](./sandbox-hitl-approval-policy-proposal.md)、[`approval-model-claude-code-vs-agent-diva.md`](./approval-model-claude-code-vs-agent-diva.md)、M3（GMH-30..33）
  - Plan 工作流：[`plan-mode-workflow/plan-mode-workflow-research.md`](./plan-mode-workflow/plan-mode-workflow-research.md)
- 参考源码（只读对照）：
  - `morediva/.workspace/hermes-agent/tools/clarify_tool.py`
  - `morediva/.workspace/claude-code/packages/builtin-tools/src/tools/AskUserQuestionTool/`
  - `morediva/.workspace/OpenHarness/src/openharness/tools/ask_user_question_tool.py`

---

## 1. 现象与问题定性

### 1.1 用户侧对话

用户追问「不是应该使用你的询问工具吗」后，Agent 回复大意：

- 当前环境只有文件、网页搜索、定时任务等工具；
- **没有专门的「表单/问卷」交互工具**；
- 改用纯对话一题一题提问。

### 1.2 结论先行

这不是「模型偷懒没调工具」的表层问题，而是 **能力缺失 + 提示词误导** 的结构性缺口：

| 层次 | 现状 | 影响 |
|------|------|------|
| 工具层 | **不存在** `clarify` / `ask_user_question` 类工具 | LLM 无法选择「询问工具」 |
| 装配层 | `MessageTool` 有实现，但 **未接入** `ToolAssembly` | 连 channel 转发 message 也不在默认工具表 |
| 提示词 | system prompt **禁止** 正常对话走 `message` 工具 | 模型被引导「直接回文本」 |
| 运行时 | 无「提问 → 挂起 → 用户答 → 恢复 tool result」闭环 | 即使有工具名，也无法真正阻塞等待 |
| GUI/CLI | 无提问卡 / 选项 UI / 回答回传契约 | 表面无法形成交互闭环 |
| 术语 | 仓库内 **M3 HITL** 指 **审批治理**（Plan/Sandbox/Memory），不是主动提问 | 易把两套 HITL 混为一谈 |

**一句话**：Agent 说「没有询问工具」在当前产品里是**事实正确**的；缺口在产品能力，而非单次会话模型失误。

---

## 2. 代码证据（只读勘察，2026-08-05）

### 2.1 内置工具清单中没有询问类工具

`agent-diva-tools` 现有模块：filesystem / shell / web / cron / spawn / message / planning / update_plan / execution_todo / mcp / attachment 等。

**没有** `clarify.rs`、`ask_user*.rs` 或等价模块。

`BuiltInToolsConfig`（`agent-diva-agent/src/tool_config/builtin.rs`）字段：

- filesystem, shell, web_search, web_fetch, spawn, cron, mcp, attachment, enqueue_background_task, update_plan  
- **无** `message` / `clarify` / `ask_user` 开关

### 2.2 ToolAssembly 未注册 MessageTool

`agent-diva-agent/src/tool_assembly.rs` 的 `build_internal` 只注册：

- 文件系统、附件、exec、web、spawn、mcp、cron、background task、custom_tools、update_plan、execution todo

`MessageTool` 仅在 `agent-diva-tools` 中实现并 `pub use`，全仓无生产路径 `registry.register(MessageTool...)`。

### 2.3 System prompt 主动压制「工具式沟通」

`agent-diva-agent/src/context.rs`：

- 能力列表写了 “Send messages to users on chat channels”；
- 但末尾强制正常对话直接回文本、不要调用 `message` 工具。

结果：即使未来误注册了 `message`，**主会话调研场景仍被引导成纯文本**，不会走结构化询问闭环。

### 2.4 子代理提示也承认无 message

`agent-diva-agent/src/subagent.rs` 含 “no message tool available”。

### 2.5 现有 HITL 是「审批」，不是「提问」

- `TODOLIST` **M3 HITL（GMH-30..33）**：统一审批协调器、receipt、GUI 审批抽屉、CLI queue——域是 **Plan / Sandbox Command / Memory apply**。
- `docs/dev/governance-m3-goal/` **不覆盖** Agent 主动 clarify。
- Command 审批已有可借鉴模式：`CommandApprovalCoordinator` 在 `exec` 工具 `execute()` 内 **oneshot 阻塞等待用户决策**。

### 2.6 历史文档曾误判「已有 clarify 等效」

归档 gap 分析（`docs/dev/archive/.../openharness-vs-diva-pro-gap-analysis.md`）曾写：Ask User Question「有 clarify 等效但可能不够灵活」。

**本次核实：该判断错误。** 当前主干既无 Hermes 式 `clarify`，也无 OpenHarness 式 `ask_user_question`，亦无 Claude Code 式 `AskUserQuestion`。

---

## 3. 参考项目对照

### 3.1 Hermes Agent — `clarify`

- 路径：`.workspace/hermes-agent/tools/clarify_tool.py`
- 参数：`question`（必填）、`choices`（可选，最多 4 项）
- 机制：平台注入 `callback(question, choices) -> str`；无 callback 时返回不可用
- Gateway：`send_clarify` + 文本拦截实现 messaging 闭环
- 子代理黑名单含 `clarify`

### 3.2 Claude Code — `AskUserQuestion`

- 多题、多选、推荐项置顶 + `(Recommended)`、始终可 Other
- Plan 模式：可澄清需求；**禁止**用它问「计划好了吗」——批准走 `ExitPlanMode`

### 3.3 OpenHarness — `ask_user_question`

- `ask_user_prompt` 从 execution metadata 注入；无 prompt 时 unavailable
- e2e：`ask_user_flow` / React TUI question modal

### 3.4 可迁移点

| 参考点 | 建议吸收 | 不建议照搬 |
|--------|----------|------------|
| 平台 callback 注入 | 与 `CommandApprovalCoordinator` 同构 | Hermes gateway 文本拦截细节可后置 |
| 结构化 options + Other | GUI 体验核心 | 首版不必 HTML preview |
| 子代理禁用 | 与 diva subagent 权限模型一致 | — |
| Plan 澄清 vs 审批分契约 | 勿与 Plan approval receipt 混状态机 | — |
| 阻塞式 tool result | MVP 最简单闭环 | 首版不必 durable 跨重启 |

---

## 4. 两套「人在回路」

```text
A. Governance HITL（已有 / M3）
   触发：危险命令 / Plan 批准 / Memory apply
   形态：审批卡、approve/edit/reject、receipt、ledger
   目标：安全与治理

B. Conversational Clarify HITL（缺失，本提案）
   触发：Agent 主动需要偏好/歧义/决策/调研
   形态：问题卡、选项/自由输入、回答回 tool result
   目标：信息获取与协作闭环
```

**不得**把 B 硬塞进 Plan/Memory/Command 审批域；可 **复用**「pending → 用户响应 → 恢复执行」的工程模式，但 **独立工具名、独立事件、独立 UI**。

---

## 5. 根因链

```text
用户期望「使用询问工具」
        │
        ▼
注册表中无 ask/clarify 工具  ──────────────► 模型看不到 function schema
        │
        ▼
MessageTool 未装配 + prompt 禁止 message ──► 无法「伪询问」
        │
        ▼
无 AskUserCoordinator / GUI 提问投影 ─────► 即使硬调工具也无法等待答案
        │
        ▼
模型只能纯文本提问 ───────────────────────► 用户感知「HITL 未闭环」
```

---

## 6. 产品提案

### 6.1 目标行为（验收叙事）

1. Agent **调用** `ask_user`（建议名），而非只生成静态问卷文件或纯文本长问卷；
2. 运行时 **挂起当前 tool call**，向 GUI/CLI 推送结构化问题；
3. 用户选择选项或填写 Other；
4. 答案作为 **tool result** 返回；
5. Agent **继续同一 turn** 后续推理。

### 6.2 命名

| 候选名 | 优点 | 缺点 |
|--------|------|------|
| `ask_user`（推荐 MVP） | 短、清晰 | 与审批 “ask” 略近 |
| `clarify` | 与 Hermes 对齐 | 语义偏澄清 |
| `ask_user_question` | 与 OpenHarness/Claude 对齐 | 偏长 |

### 6.3 工具契约（草案）

MVP 可缩为 Hermes 级单题 `question` + `choices[]`；完整草案支持最多 4 题、每题最多 4 选项、`multi_select`、`allow_other`、可选 `context`。

结果：`status: answered | cancelled | timeout | unavailable`，`answers[]` 含 `selected` / `other_text`。

### 6.4 使用策略

**应该用**：需求歧义、trade-off、偏好调研、关键决策；Plan 探索澄清目标。

**不应该用**：危险命令确认（sandbox approval）；Plan 批准执行；低风险可自决细节；子代理默认禁用。

### 6.5 与 MessageTool

| 工具 | 用途 |
|------|------|
| `message` | 向其他 channel 旁路推送，不阻塞回填本 tool call |
| `ask_user` | 当前会话结构化提问并 **阻塞等待** 答案 |

**不要**用复活 `message` 冒充 HITL 询问。

---

## 7. 技术架构提案

### 7.1 形态

```text
LLM tool_call(ask_user)
        → AskUserTool::execute
        → AskUserCoordinator.request  ──►  GUI/CLI 问题卡
        ← oneshot answer
        → tool result JSON → 下一轮 LLM
```

### 7.2 分层

| 层 | 职责 |
|----|------|
| `agent-diva-tools` | `AskUserTool` schema + 校验 + coordinator |
| core/agent | `AskUserCoordinator`、超时/取消 |
| `ToolAssembly` + `BuiltInToolsConfig` | 注册；subagent 关闭 |
| `context` system prompt | 引导关键决策用工具 |
| bus 事件（可选） | `UserQuestionRequested` / `Answered` |
| Manager / GUI / CLI | 问题卡与回传；headless → unavailable |

### 7.3 与治理 HITL 边界

| 维度 | 审批 | ask_user |
|------|------|----------|
| 持久化 | durable ledger / receipt | MVP 内存 oneshot |
| 重启 | Pending 恢复 | MVP 超时/取消，不跨进程 |
| UI | 审批抽屉/就地卡 | Chat 内 QuestionCard |

### 7.4 上下文策略

| 上下文 | ask_user |
|--------|----------|
| 普通聊天 | 允许（默认开） |
| Plan 探索 | 允许（澄清） |
| Plan 等待批准 | 禁止替代批准 |
| Execution | 首版可禁止以降复杂度 |
| Subagent / background / cron | 禁止或 unavailable |
| Mask read-only | 允许（无副作用信息获取） |

---

## 8. 分阶段路线图

### Phase 0 — 研究与立项（本回合完成归档）

- [x] 根因确认  
- [x] 与 M3 Governance HITL 划界  
- [x] 对照 Hermes / Claude Code / OpenHarness  
- [x] 写入 `docs/research` + `TODOLIST`  
- [x] 用户确认命名、单题 vs 多题、GUI 是否同迭代  
  （2026-08-05 拍板：工具名 `ask_user`；单题单轮（Hermes 级）；
  Phase 1 仅运行时，GUI/CLI 卡下一迭代；挂起默认超时 10 分钟）

### Phase 1 — 运行时 MVP

1. `AskUserCoordinator`（内存 oneshot）  
2. `AskUserTool` + 配置 + 装配  
3. system prompt / tool description  
4. 单元与 mock 集成测试  

### Phase 2 — 表面闭环

1. [x] Manager HTTP API + 注入链（`2b6283c8`：3 端点 + AppState/bootstrap 注入）  
2. [x] GUI QuestionCard（`cf268e5e`：AskUserQuestionCard 聊天内联 + 2s 轮询；Tauri 桥 `342807a1`）  
3. [x] CLI interactive（`be43c130`：AskUserAnswerer + dialoguer；TUI 内联）  
4. [ ] 人工 smoke：互动调研必须出现 tool call（步骤见
   `docs/logs/2026-08-ask-user-hitl-research/v0.2.0-ask-user-surface/acceptance.md`）

### Phase 3 — 策略硬化

Plan 矩阵、subagent 黑名单、UI 与审批区分、可选 messaging clarify。

### MVP 外不做

- 写入 governance.db / 统一审批抽屉  
- 跨重启恢复未答问题  
- HTML preview  
- 用 `message` 冒充闭环  

---

## 9. 验收矩阵（实现时）

| ID | 场景 | 期望 |
|----|------|------|
| A1 | tool definitions | 含 `ask_user` |
| A2 | mock LLM 应问场景 | tool_call 而非仅 final text |
| A3–A4 | 选项 / Other | 正确回填 |
| A5–A6 | 取消 / 超时 | 不挂死 |
| A7 | headless | unavailable |
| A8 | subagent | 未注册或拒绝 |
| A9 | 危险命令 | 仍走 exec approval |
| A10 | 静态问卷文件 alone | 不作为「已完成调研」充分条件 |

**用户原对话级验收**：多选题调研必须出现 `ask_user` tool call；GUI/CLI 可点选；答完后 Agent 基于答案继续，而不是声称「没有询问工具」。

---

## 10. 开放问题（实现前拍板）

1. ~~MVP 深度：仅运行时 mock，还是同迭代 GUI？~~ → 仅运行时（Phase 1），GUI/CLI 卡 Phase 2
2. ~~单题（Hermes）vs 多题（Claude）？~~ → 单题单轮（Hermes 级）
3. ~~默认超时（建议 5–15 分钟）与取消策略~~ → 10 分钟（`DEFAULT_ASK_USER_TIMEOUT`），可配置；
   取消/超时均不挂死，返回 `cancelled` / `expired` 状态
4. 问答正文是否进审计日志（建议：MVP 不进，后续按隐私策略评估）
5. 有工具后仍可能纯文本偷懒——需 prompt + 可选评测约束

> **实现注意（2026-08-05 Phase 1 勘察补充）**：
> `ToolRegistry` 有全局执行超时（默认 120s，
> `agent-diva-tooling/src/registry.rs:41`），会杀死阻塞等待的 `ask_user`。
> `Tool` trait 预留了 `timeout_secs()` 覆盖（`agent-diva-tooling/src/base.rs:18`，
> 注释即 "long-lived interactive tools"）。`AskUserTool` 必须实现
> `timeout_secs()` 返回协调器超时（默认 600s），否则提问超过 2 分钟会被强制超时。
> 另外 `ToolConfig.ask_user` 默认 `None`（headless → unavailable），由运行入口显式注入。

---

## 11. 结论

| 问题 | 答案 |
|------|------|
| 为什么没用询问工具？ | **产品未提供**，不是偶发未调用 |
| 是否与 M3 HITL 同一 bug？ | **否**。M3=审批治理；本缺口=Conversational Clarify |
| MessageTool 能否顶替？ | **不能** |
| 修复方向 | 新增 `ask_user` + Coordinator + 装配/prompt + GUI/CLI |

实现请开独立回合，按 Phase 1→2→3 推进，并与 `审批三模式完善` / M3 审批验收 **分轨**。
