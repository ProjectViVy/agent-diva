# Summary — ask_user 运行时 MVP（CLARIFY-HITL Phase 1）

- 版本：`v0.1.0-ask-user-runtime-mvp`
- 日期：2026-08-05
- 类型：运行时能力实现（Coordinator + Tool + 装配 + prompt + 测试）

## 背景与拍板

TODOLIST `CLARIFY-HITL`（sev-P1）：Agent 无「询问工具」，调研场景退化为纯文本。
2026-08-05 用户拍板：工具名 `ask_user`；单题单轮（Hermes 级）；Phase 1 仅运行时
（GUI/CLI 卡下一迭代）；挂起默认超时 10 分钟。提案：
`docs/research/ask-user-clarify-hitl-proposal.md`（Phase 0 已勾选，§10 已记录决策）。

## 做了什么

1. **`AskUserCoordinator`**（`agent-diva-core/src/ask_user.rs`，新模块）：
   进程内内存 oneshot；`request` 阻塞等待 → `pending` 查询 → `answer`（选项/Other）
   → `cancel`；超时返回 `expired`；默认 10 分钟（`DEFAULT_ASK_USER_TIMEOUT`）。
2. **`AskUserTool`**（`agent-diva-tools/src/ask_user.rs`，新模块）：
   schema `question`（必填）+ `choices`（≤4）+ `allow_other` + `context`；
   无 coordinator 时返回 `{"status":"unavailable"}`（headless 降级）；
   **实现 `timeout_secs()` 覆盖 registry 全局 120s 超时**（勘察发现的盲点，
   已补入提案 §10 与 TODOLIST）。
3. **装配**：`BuiltInToolsConfig`（agent 侧 + core schema 用户配置层）新增
   `ask_user` 开关（默认开）；`ToolAssembly::with_ask_user_coordinator` 注册；
   subagent（`for_subagent`）与 execution session 不注册；`ToolConfig.ask_user`
   默认 `None`（headless → unavailable），manager/CLI 构造点已同步；
   `AgentLoop::set_ask_user_coordinator` 提供运行入口（Phase 2 表面层使用）。
4. **prompt**：`context.rs` 能力列表新增 `ask_user`，并引导「需求用户输入继续的
   关键场景用 ask_user 而非纯文本列问题」；`subagent.rs` 提示同步声明无 ask_user。
5. **测试**：core 7 个单元测试（选项/Other/非法输入保留挂起/取消/超时/NotFound/
   默认超时）；tools 5 个（unavailable/缺参/超限/答案回填/timeout_secs）；
   tool_assembly 5 个（注册/headless/subagent 排除/execution 排除/协调器闭环）；
   agent_loop 1 个 mock 集成测试（A2：tool_call → 挂起 → answer → tool result
   回填 → 下一轮 LLM）。

## 影响范围

- `agent-diva-core`：新增 `ask_user` 模块；`config/schema.rs` builtin 开关。
- `agent-diva-tools`：新增 `ask_user` 模块；lib 导出。
- `agent-diva-agent`：`tool_assembly.rs`、`tool_config/builtin.rs`、`agent_loop.rs`、
  `context.rs`、`subagent.rs`。
- `agent-diva-manager` / `agent-diva-cli`：构造点同步（默认 headless）。
- 文档：提案 Phase 0/§10、TODOLIST、LOCK、本日志。

## 未做（下一迭代）

- GUI QuestionCard / AgentEvent 投影 / CLI interactive（Phase 2）。
- Plan 矩阵硬化、子代理策略细节（Phase 3）。
- 问答正文审计日志（§10 开放问题 4，MVP 不进）。
