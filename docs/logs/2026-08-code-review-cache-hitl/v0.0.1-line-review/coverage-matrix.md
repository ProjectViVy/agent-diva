# Coverage Matrix — 功能 × 证明层级

证明层级：`U` 单元 · `I` 集成/crate e2e · `F` 前端组件测试 · `S` CLI/Manager 自动化 · `M` 仅人工 acceptance · `∅` 无

基线：`b4d2a84b` HEAD（**不含** `feat/m3-hitl-closure`）

---

## Track A — 缓存与工具调用链

| 能力 | 运行时 | GUI | CLI | Manager | 证明 | 备注 |
|------|--------|-----|-----|---------|------|------|
| 稳定前缀 / CORE schema 字节稳定 | 有 | n/a | n/a | n/a | **U** registry + assembly | C1 |
| session-stable section cache | 有 | n/a | n/a | n/a | **U** C1 日志 T1–T8 | 无 GUI 设置面 |
| prompt cache 观测 / final-wire | 有 | n/a | log | n/a | **U** final_wire + provider tests | DeepSeek core hash 弱（S2） |
| 分层 ContextBudget 报告 | 有 | 上下文环 % | n/a | n/a | **U** budget | GUI 仅粗粒度 history 估计 |
| 大结果 → artifact ref + preview | 有 | 原始 JSON/截断 160 字 | 同 transcript | n/a | **U** tool_results / artifact | **Gap：** 不解析 preview 字段 |
| `read_tool_result` | CORE 工具 | 无按钮 | 模型可调 | n/a | **U** read_tool_result | 仅模型侧 |
| microcompact 旧 inline | 有 | 不可见 | 不可见 | n/a | **U** tool_results | 压力触发 |
| canonical checkpoint | 有 | 无专用 UI | 无 | n/a | **I** compaction_* 精简后 | 长任务「卡住」可能误判（Gap） |
| tool_search 自动激活 ≤8 | 有 | n/a | n/a | n/a | **U/I** registry + agent_loop | 无 UI 可接受 |
| 未激活 deferred 不可执行 | 有 | n/a | n/a | n/a | **U** not_active | |
| 授权/mask 仍门禁 | 有 | n/a | n/a | n/a | **U** 部分 | 与 plan phase 交叉 |
| mount_tool 协议 | **已删除** | — | — | — | 符号扫描 ∅ in `*.rs` | 文档仍提及（S2） |
| artifact 重启/TTL/越权 | 有 | n/a | n/a | n/a | **U** tool_artifact 模块测 | |
| 子 agent artifact session | task_id 绑定 | n/a | n/a | n/a | 点检 assembly | |

### Track A 前端结论

- **无**「缓存开关 / artifact 浏览器 / compact 进度」产品面；属 runtime。
- **有缺口：** tool row 把 artifact JSON 当普通字符串截断展示，用户难读 preview、不知可 `read_tool_result`。
- Chat 上下文环与 CTX budget 分层报告**未对齐**（仅 history 估计）。

---

## Track B — HITL（HEAD 实际能力）

| 能力 | GUI | CLI | 事件/API | 证明 | 备注 |
|------|-----|-----|----------|------|------|
| 统一审批中心 Drawer | ApprovalCenter* | approvals 命令族（部分） | `approval-event` + list API | **F** Drawer/Card tests | 主路径 |
| legacy command-approval SSE | **不消费** | — | 仍 start + emit | ∅ 前端 | 半残留（S2） |
| Chat 内联审批卡 | **无** | — | — | ChatView.test 期望 0 Card | 去重方向正确 |
| 三模式切换 UI | 有（默认 smart） | approval-mode | send_message `approvalPolicy` | **F** 少量 | **不持久**（S2） |
| 三模式后端区分 | 半残 | 半残 | orchestrator policy | sandbox U 旧语义 | **M3 未合入**（S1） |
| Trusted 自动学习 Allow | **无（未合入）** | — | — | 仅侧分支 | |
| Plan 审批卡 | PlanApprovalCard | plan cmds | planning | **F** | 分轨 |
| ask_user 问题卡 | AskUserQuestionCard | interactive chat/tui | Manager HTTP + 2s poll | **F** + **U** + CLI tests | 人工 smoke 仍开 |
| ask_user 超时 10min | coordinator | 同 | — | **U** timeout_secs | |
| ask_user allow_other 默认 true | 卡支持 | 同 | — | **U** + F free-text | |
| 重启 pending HITL store | N/A on HEAD | — | — | 侧分支测试 | 统一 spine 未合入 |
| 审批超时倒计时 UX | **无** | — | Expired | ∅ | TODOLIST 已知 Gap |
| Memory 治理审批 | Evolution/Memory | — | laputa | 部分 F | 交叉主线 |

### Track B 侧分支（未合入，不计入 HEAD 证明）

| 能力 | 分支 | 证明声称 |
|------|------|----------|
| M3 S1 rule store 共享 | feat/m3-hitl-closure | sandbox tests |
| M3 S2 三模式 review 拆分 | 同上 | three-mode contract tests |
| M3 S3 Guardian 生产接线 | 同上 | sandbox + tools |
| M3 S4 trusted 学习 | 同上 | |
| M3 S5 GUI 模式持久化 | 同上 | ChatView.test |
| 2026-07 durable interaction store | refactor/deep-governance | restart test |

---

## 人工 smoke 仍打开（TODOLIST 对齐）

| 项 | 状态 |
|----|------|
| M3 审批 HITL 集中人工 smoke | 待办；且代码未合入主干时 smoke 无意义 |
| CLARIFY-HITL 真实 LLM 触发 | 待办；协议层已有 U/F/S |
| 沙箱设置保存桌面 smoke | 待办（并行） |

---

## 覆盖评分（主观）

| 域 | 运行时正确性测试 | 前端功能覆盖 | 端到端产品证明 |
|----|------------------|--------------|----------------|
| Track A | **强** | **弱** | **中**（无长任务桌面） |
| ask_user | **中强** | **中**（有卡+测试，无 LLM e2e） | **弱**（人工） |
| 审批三模式 | **弱（HEAD）** | **中（有 UI）** | **弱** |
| 统一审批 Drawer | **中** | **中强** | **弱**（人工） |
