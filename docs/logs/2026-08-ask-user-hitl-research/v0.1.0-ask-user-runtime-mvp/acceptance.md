# Acceptance — ask_user 运行时 MVP（Phase 1）

- 版本：`v0.1.0-ask-user-runtime-mvp`
- 日期：2026-08-05

## 验收步骤（产品/用户视角）

1. **工具存在**：主会话 tool definitions 含 `ask_user`（schema：
   `question` + `choices`≤4 + `allow_other` + `context`）。
2. **询问闭环（mock 验证）**：Agent 调用 `ask_user` → 运行时挂起该 tool call →
   协调器暴露 `pending` → 用户（或 Phase 2 表面层）通过 `answer` 回填 →
   答案作为 tool result 返回 → Agent 继续同一 turn 后续推理。
3. **选项与 Other**：`selected` / `selected_index` / `other_text` 正确回填；
   `allow_other=false` 时 Other 被拒绝且问题保持挂起。
4. **不挂死**：用户取消（`cancelled`）或超时（默认 10 分钟，`expired`）均释放
   turn；registry 全局 120s 超时被 `timeout_secs()` 覆盖。
5. **headless 降级**：无 coordinator 注入时返回 `{"status":"unavailable"}`，
   不阻塞。
6. **子代理隔离**：subagent 工具表无 `ask_user`。
7. **用户原对话级验收（Phase 2 完成后）**：多选题调研必须出现 `ask_user`
   tool call；GUI/CLI 可点选；答完后 Agent 基于答案继续，而不是声称
   「没有询问工具」。

## 当前状态

- 运行时层（1–6）本迭代完成，验收记录见 `verification.md`。
- 第 7 项依赖 Phase 2（GUI QuestionCard / CLI interactive）与人工 smoke。
