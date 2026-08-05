# Summary — ask_user 表面闭环（CLARIFY-HITL Phase 2）

- 版本：`v0.2.0-ask-user-surface`
- 日期：2026-08-05
- 类型：表面闭环实现（Manager API + CLI interactive + GUI QuestionCard）

## 背景与拍板

Phase 1（`v0.1.0`）交付运行时 MVP 后，`ToolConfig.ask_user` 三处注入点均为
`None`（headless → `unavailable`），用户无法在 GUI/CLI 上回答问题。2026-08-05
拍板：Phase 2 完整做；GUI 用前端 2s 轮询发现挂起问题（不做 SSE 端点）。

## 做了什么（4 个 commit）

1. **`2b6283c8` feat(manager)**：`handlers/ask_user.rs` 新建 3 端点
   （`GET /api/ask-user/questions`、`POST .../:id/answer`、`POST .../:id/cancel`）；
   AppState 加 `ask_user` 字段（生产链 `new_with_runtime_governance` 注入，测试入口
   签名不变）；bootstrap → GatewayBootstrap → build_agent_loop（`ask_user: None` →
   `Some`）→ task_runtime 全链路注入；server router 注册。7 个 handler 测试。
2. **`be43c130` feat(cli)**：`build_local_cli_agent` 创建并注入 coordinator；
   `AskUserAnswerer` 后台任务轮询 `pending()`，dialoguer Select（choices）/
   Input（Other）交互回答；非 TTY 自动 cancel；每轮 turn 后清理残留问题；
   TUI 模式 timeline 内联展示 + 输入行回答（数字=选项，allow_other 自由文本）。
   2 个 headless 单测。
3. **`342807a1` feat(gui)**：Tauri command 桥
   `list_ask_user_questions` / `answer_ask_user_question` / `cancel_ask_user_question`
   （HTTP 代理 manager 端点），invoke handler 注册。
4. **`cf268e5e` feat(gui)**：`AskUserQuestionCard.vue`（问题/选项按钮/Other 输入/
   取消）；App.vue `pendingQuestions` 状态 + 2s 轮询 + answer/cancel 处理器；
   NormalMode/ChatView props/emits 透传，聊天内联渲染。4 个 vitest 组件测试。

## 影响范围

- `agent-diva-manager`：新 handler、AppState/注入链、router。
- `agent-diva-cli`：chat_commands（answerer）、main（TUI）。
- `agent-diva-gui`：src-tauri commands + 前端组件/App/NormalMode/ChatView。
- 文档：提案 §8 Phase 2、TODOLIST、LOCK、本日志。

## 未做（下一迭代）

- Phase 3 策略硬化：Plan 矩阵细化、subagent 黑名单断言、UI 与审批抽屉区分
  （当前为聊天内联卡，符合提案 §7.3 方向）。
- 问答正文审计日志（提案 §10 开放问题 4，MVP 不进）。
- 人工 smoke 的 GUI 实机验证（本迭代记录见 `verification.md`，需真实 Tauri
  桌面 + 真实 LLM 触发 ask_user 的最终验收）。
