# Findings — 缓存/工具调用链 + HITL 逐行审查

基线：`b4d2a84b` (`agent-diva-pro`) · 2026-08-12

Severity：S0 数据/安全致命 · S1 主路径错误或交付事实错误 · S2 边界/空洞 · S3 可维护 · Gap 产品覆盖缺口

---

## S1 — 主路径 / 交付真相

### [S1] M3 HITL S1–S5 未合入 agent-diva-pro，但被当作「最近大型 commit」存在

- Track / Slice: B3
- Location: commits `2c8db02a`…`defb84fd` 仅在分支 `feat/m3-hitl-closure`；`git merge-base --is-ancestor 9fb02d8a HEAD` → fail
- Evidence: HEAD `git log -- agent-diva-sandbox/src/guardian.rs` 最新仍为 `caee0020` / `2eb38b3e`（2026-06）；`git blame` 显示三模式 match 臂自 6 月未变。侧分支 tip `defb84fd` 含完整 S1–S5。
- Impact: 审查/验收若默认「HEAD = 最新 HITL」，会**误判三模式、自动学习、GUI 模式持久化已上线**。LOCK/迭代日志关闭 M3 时易误导并行会话。
- Frontend?: yes — S5 持久化仅在侧分支 ChatView
- Suggested fix: 合并或显式标注「未合入」；更新 TODOLIST/LOCK handoff
- Confidence: **high**

### [S1] 生产 Shell 路径未挂载 mode-driven Guardian（HEAD）

- Track / Slice: B3 / 交叉
- Location: `agent-diva-tools/src/shell.rs` `with_approval_backend` ~103–118
- Evidence: HEAD 仅 `ToolOrchestrator::new(manager, approval_policy)`，无 `GuardianConfig::for_ask` / `with_guardian_and_exec_policy`。侧分支 `feat/m3-hitl-closure` 的 S3 commit `a37f0f0c` 才接线。
- Impact: GUI 传递 `approvalPolicy`（cautious/smart/trusted → on-request/on-failure/unless-trusted）后，**Guardian 风险预判与自动学习不参与生产 shell**；行为退回 orchestrator + coordinator 旧语义。
- Frontend?: yes — 用户切换三模式时体感与文案不符
- Suggested fix: 合入 S3 或在 HEAD 最小移植 Guardian wiring
- Confidence: **high**

### [S1] Guardian review 三模式在 HEAD 仍合并为同一逻辑臂

- Track / Slice: B3
- Location: `agent-diva-sandbox/src/guardian.rs` ~343–389
- Evidence:
  - `can_skip()` 无条件 Defer（S2 曾改为仅 Never 短路，HEAD 无此改动）
  - `OnFailure` → 盲 `Defer`（「智能」不做风险预判）
  - `OnRequest | UnlessTrusted` 同一分支，未知命令一律 `require_approval`（「信任」不放行未知）
- Impact: 即使将来仅改 config flags，**UnlessTrusted 与 OnRequest 的 review 策略仍无法区分**；与 research `approval-model-claude-code-vs-agent-diva.md` 及 TODOLIST「审批三模式完善」描述一致，且 **S2 修复未在主干**。
- Frontend?: yes
- Suggested fix: 合入 `9fb02d8a` 的 review 拆分 + `GuardianConfig::for_ask`
- Confidence: **high**

---

## S2 — 边界 / 通道 / 可观测

### [S2] Legacy 命令审批 SSE 仍启动并 emit，前端已不监听

- Track / Slice: B1 表面
- Location:
  - `agent-diva-gui/src/App.vue` ~2002–2033：只 `listen('approval-event')`，同时 `start_command_approval_stream` + `start_approval_stream`
  - `agent-diva-gui/src-tauri/src/commands.rs` ~2979–3025：仍 emit `command-approval-requested`
  - ChatView **无** ApprovalBanner / 内联 ApprovalCenterCard
- Evidence: `rg command-approval-requested` 在 GUI 仅 commands.rs 发射侧；App.vue 无 listener。
- Impact: 双通道**半残留**——不会像旧版三重卡片那样重复渲染（好），但浪费连接/日志，且 `capabilities.ts` 仍文档化 legacy entrypoint，增加维护歧义。若未来有人重新 listen legacy，会回潮重复 UI。
- Frontend?: yes
- Suggested fix: 删除或 feature-gate `start_command_approval_stream`；统一只走 approval-event
- Confidence: **high**

### [S2] GUI permissionMode 会话内默认 smart，不持久化（HEAD）

- Track / Slice: B3
- Location: `ChatView.vue` ~200：`ref(...'smart')`，无 localStorage
- Evidence: 侧分支 S5 有 `PERMISSION_MODE_KEY` + watch；HEAD 无。`App.vue` sendMessage 会映射 policy，但刷新丢失。
- Impact: 用户选「谨慎/信任」后重启 GUI 回到 smart，可能在不知情下改变审批严格度。
- Frontend?: yes
- Suggested fix: 移植 S5 或最小 localStorage
- Confidence: **high**

### [S2] 无 prompt-cache 的提供商 final-wire `core_tools_hash` 常为空

- Track / Slice: A4
- Location: `agent-diva-providers/src/final_wire.rs` ~30–34；`openai_compatible.rs` ~864–866（`supports_cache_control` 为 false 时不 `apply_cache_control`）
- Evidence: `snapshot_from_wire` 用**第一个**带 `cache_control` 的 tool 下标作 CORE 终点；无标记时 `core_end = 0` → `core_tools_hash = hash([])`。DeepSeek 等 `supports_prompt_caching=false` 路径验证见同文件测试 `test_supports_cache_control_deepseek_false`。
- Impact: 非 Anthropic 类缓存提供商上 **CORE 前缀指纹失效**，C5a 可观测性对默认 DeepSeek 路径弱；不破坏正确性（observer 非权威），但排障误导。
- Frontend?: no
- Suggested fix: 无 cache_control 时回退 `core_count` 显式传入，或 hash 全 CORE 切片由 agent 侧 boundary 提供
- Confidence: **high**

### [S2] Microcompact 强制 status=`ok` 且仅跳过 `Error:` 前缀

- Track / Slice: A2
- Location: `agent-diva-agent/src/tool_results.rs` ~117–134
- Evidence: 非 `Error:` 开头、>4k、非已有 ToolResultRef 的 tool 消息一律 `persist_artifact_result(..., "ok", ...)`
- Impact: 非标准错误格式的大失败结果可能被标为 ok artifact；一般路径 tool_step 错误走 inline 且常带 `Error:`，风险中等。
- Frontend?: no
- Suggested fix: 解析既有 status / is_error 元数据；失败不 microcompact 为 ok
- Confidence: **med**

### [S2] 文档与 C4 acceptance 仍描述已删除的 mount_tool / tool_discovery_v1

- Track / Slice: A3/A8
- Location: `docs/logs/.../v0.0.9-c4-deferred-tool-discovery-recall/*`、部分 research；生产 `*.rs` 中 `mount_tool|tool_discovery_v1` **零匹配**（clean break 成功）
- Impact: 人工验收按旧文档会失败；新会话误实现 mount 协议。
- Frontend?: no
- Suggested fix: 文档标 superseded by C5e；acceptance 改 auto-activate
- Confidence: **high**

---

## S3 — 可维护 / 防御深度

### [S3] `format_messages_for_compaction` 折叠工具组时不检查 `group.complete`

- Track / Slice: A5
- Location: `compaction_exec.rs` ~254–261 vs `select_safe_compaction_end` ~409–412
- Evidence: 安全边界选择会排除 incomplete；format 自身若被误用会把未完成组标成 `status=completed`
- Impact: 当前调用链安全；防御深度不足
- Suggested fix: format 内 `if !group.complete { 逐条输出 }`
- Confidence: **med**

### [S3] 2026-07 durable HITL interaction store 不在 HEAD

- Track / Slice: B1
- Location: commits `014e4347`…`bf6f6e8d` 在 `refactor/deep-governance`
- Evidence: HEAD 无 `interaction_store` 符号；审批走 unified ApprovalCenter（`44b893ce` 等）
- Impact: 不是回归，但是「统一 HITL product loop」叙事分裂为两套历史；重启中断证明测试不在本分支主线
- Suggested fix: 文档标明权威路径为 ApprovalCenter + ask_user coordinator
- Confidence: **high**

---

## 正向结论（非问题）

1. **C5e clean break 成功：** 生产 Rust 无 `mount_tool` / `tool_discovery_v1` / `tool_not_discovered`；`tool_search` → `ActiveDeferredTools::replace`（上限 8）→ 下一次 definition set 暴露；未激活 deferred execute → `not_active` / `ToolUnavailable`。
2. **Artifact 安全模型扎实：** session digest 路径、跨 session Forbidden、SHA 校验、TTL GC、容量驱逐、物化失败显式 error JSON、无静默截断成功结果。
3. **Canonical 单一表示在 tool_step：** 成功走 `canonicalize_tool_result`，错误 inline；materialization failure 提升 is_error。
4. **Checkpoint 未决工具成组：** `select_safe_compaction_end` + 测试 `incomplete_tool_group_stays_out_of_prefix`；与 HITL 挂起（ask_user 未决 tool pair）兼容。
5. **ask_user 超时正确：** `DEFAULT_ASK_USER_TIMEOUT=600`，`AskUserTool::timeout_secs` 覆盖 registry 120s；GUI 卡 + CLI answerer + Manager HTTP 均在 HEAD。
6. **Final-wire 在 Anthropic/带 cache 的 OpenAI 兼容路径：** agent 侧 `apply_core_tool_cache_anchor` 标记 CORE 末工具；deferred 后缀不进入 core hash（有单测）。
7. **审批 UI 三重显示：** HEAD ChatView 已无内联 ApprovalCenterCard/Banner；统一 Drawer（相对 v0.5.1 目标，主渲染点收敛）。

---

## 交叉问题

| 交叉 | 结论 |
|------|------|
| C5e 自动激活 ≠ 批准 | registry 只控制 schema 可见与 not_active；shell/exec 仍走 orchestrator。但 **Guardian 未接线** 时「批准」语义偏弱。 |
| Checkpoint × ask_user 挂起 | 未完成 tool group 不进 compact 前缀；microcompact 保护最近 assistant tool_calls 组。 |
| permissionMode × backend | GUI 映射 policy 字符串正确；后端 Guardian 区分缺失导致 **前端有开关、后端同质**。 |

---

## 建议修复优先级

1. **P0 过程：** 标注 M3 / 旧 HITL spine 未合入（文档 + TODOLIST）。
2. **P0 功能：** 合入或重做 M3 S2+S3（Guardian 三模式 + shell 接线）。
3. **P1 UX：** permissionMode 持久化；拆除 legacy approval stream。
4. **P1 观测：** final-wire 在无 cache_control 时使用显式 core_count。
5. **P2：** GUI 解析 ToolResultRef；刷新 C4 文档；microcompact status。
