# TODOLIST

This file is the project-level backlog for bugs, gaps, and unfinished work found during development or review.

Last comprehensive scan: 2026-06-03 (`docs/dev` survey — 37 active .md files)
Last code audit: 2026-06-03 (2x Claude Code, 15 items, total cost ~$4.13)

Legend: 调研 ✅=已完成  🔄=进行中  ❌=未开始 | 代码 ✅=已实现  🟡=部分  ❌=未实现

---

## Open

### P0 — Security & Stability

- [ ] **P0-1: Infinite loop / circuit breaker** | 调研 ✅ | 代码 🟡
  Agent 在工具重复失败时可能无限循环，缺少工具调用 hash 去重和迭代预算。
  - 现存: `max_iterations=20` (agent_loop.rs:100), subagent 硬编码 15 (subagent.rs:252)
  - 缺失: 无 circuit_breaker 模块/struct, 无 tool-call hash 去重, 无连续失败检测, 无 wall-clock timeout, 无 token/cost budget
  - Source: `docs/dev/awesomeagents/unknown-deficits.md` (Defect 1), `docs/dev/awesomeagents/decisions.md` (P0-2)

- [ ] **P0-2: Sub-agent security suite** | 调研 ✅ | 代码 🟡
  缺少子代理安全控制：深度限制、凭据最小化、并发控制。
  - 现存: 硬编码 tool blacklist (for_subagent() 禁用 spawn/cron/attachment), SecurityPolicy 8 层路径校验
  - 缺失: 无 max_depth, 无 subagent 并发上限 (SubagentManager 只 track 不限流), 无 credential minimization (subagent 继承完整 API key + network config + MCP servers)
  - Source: `docs/dev/awesomeagents/decisions.md` (P0-1), `docs/dev/awesomeagents/sandbox-audit-c.md`

- [ ] **P0-3: Credential scrubbing in logs** | 调研 ✅ | 代码 ❌
  日志层没有全局凭据擦除（tracing Layer），API key 可能出现在日志中。
  - 现存: CLI `config show` 有 display-layer 脱敏 (cli_runtime.rs:405), tracing-subscriber 已就位 (logging.rs)
  - 缺失: 无自定义 tracing::Layer (零 impl Layer for), 无 regex scrubbing (sk-*/Bearer */ghp_*), `tracing-subscriber` 缺 `regex` feature
  - 泄漏点: litellm.rs debug 日志可输出 Authorization header, ErrorContext truncation 不脱敏, config schema api_key 是纯 String
  - Source: `docs/dev/awesomeagents/sandbox-audit-b.md`

- [ ] **P0-4: Session truth-source fix (Phase A-PRE)** | 调研 ✅ | 代码 ❌
  Session 持久化发生在 agent turn 太晚期，3 个 P0 data-loss bug 全部未修复。
  - Bug 1: 用户消息在 turn 完成前未持久化 → 崩溃丢失
  - Bug 2: consolidation 写 MEMORY.md/HISTORY.md 早于 session save → split-brain
  - Bug 3: `std::fs::write()` 非原子覆盖 → 写入中断 = 文件损坏 (agent-diva-files 已有 atomic temp+rename 但未被 session 复用)
  - Source: `docs/dev/agent-plan/phase-a-pre-session-truth-source-fix.md`, `docs/logs/2026-06-session-research/subagent-backend-audit.md`

- [x] **~~P0-5: Path traversal hardening~~** | 调研 ✅ | 代码 ✅ — **已实现，TODOLIST 过时**
  已全面实现 8 层路径校验 (security/path.rs): null bytes → ParentDir → URL-encoded traversal → tilde → absolute → forbidden prefix → canonicalize → symlink escape。文件工具 + shell + skill_zip 均有调用。测试覆盖: test_path_traversal_blocked, upload_skill_zip_rejects_path_traversal。

- [ ] **P0-6: Context overflow silent truncation** | 调研 ✅ | 代码 ❌
  Context 超出限制时无优雅降级，硬失败。
  - 现存: `max_tokens` (output only), `MAX_TOOL_RESULT_CHARS=80K` (blunt safety net), Usage 数据已 parse 但从未用于预算决策
  - 缺失: 无 context_budget/token_budget config, 无本地 tokenizer, 无 token counting, 无 sliding window, 无 context compaction, 无 overflow detection (靠 provider 400 error)
  - Source: `docs/dev/awesomeagents/unknown-deficits.md` (Defect 2)

### P1 — Core Infrastructure

- [ ] **P1-1: Plan+TodoList implementation** | 调研 ✅ | 代码 ❌
  6 篇设计文档已完成，0 行代码。无 planning/ 目录、无 domain types、无 SQLite schema。
  - Design docs: `docs/dev/agent-plan/` (6 files)
  - 缺失: PlanOrchestrator, PlanStateStore, PlanVerifier, PlanPhase, TodoList, TodoItem 全部不存在
  - sqlx + SQLite feature 已就位 (workspace dep)，仅 agent-diva-files 在用

- [ ] **P1-2: Phase B: Thin Observability Layer** | 调研 ✅ | 代码 🟡
  Design complete, blocks on Phase A。Tracing 基础设施完善但 spec 合规度低。
  - 现存: tracing-subscriber (EnvFilter + rolling file), trace_id (Uuid), 12 structured trace points (loop_turn.rs)
  - 缺失: 无 typed TraceId/TraceEvent, 无 JSONL writer, 无 redaction layer, 无 structured event emission, 无 debug bundle

- [ ] **P1-3: Sandbox audit remediation** | 调研 ✅ | 代码 🟡
  3 份审计报告 (A/B/C) ~20 发现待修复。注意: `agent-diva-sandbox` crate 不在当前 workspace。
  - 已实现 (core security): SecurityPolicy 8-layer, PathValidator, SecurityConfig levels, ActionTracker rate limiter, shell deny patterns, forbidden paths/extensions
  - 缺失: platform-level sandbox (RestrictedToken/Landlock/Seatbelt), env filtering, prompt injection scanning, MCP limits, subagent concurrency

- [ ] **P1-4: Permission mode UI wired to backend** | 调研 ✅ | 代码 ❌
  前端权限选择器存在但未连接后端。全代码库无 permission/approval/three-tier/sticky 实现。
  - Source: `docs/dev/awesomeagents/pro-ui-audit.md`

- [ ] **P1-5: Tool execution timeout wrapping** | 调研 ✅ | 代码 🟡
  Shell + MCP 有超时，但无全局 registry-level timeout。
  - 现存: Shell `tokio::time::timeout` built-in, MCP tool calls/list/startup 有 timeout
  - 缺失: ToolRegistry::execute() 无 `tokio::time::timeout`, filesystem/web/spawn/message 工具无超时保护
  - Quick win: 在 registry.rs:89 加 `tokio::time::timeout` 包装

- [ ] **P1-6: Error classification system** | 调研 ✅ | 代码 🟡
  ToolError 仅 5 个 flat variants，无结构化分类。
  - 现存: ToolError (5 variants, string-heavy), SecurityError (9 structured variants, has user_message + is_retryable), ErrorContext
  - 缺失: 无 error_category/ErrorKind, 无 error codes, 无 retry classification on ToolError, 无跨 crate 统一 error taxonomy

### Decisions Needed

- [ ] **D-1: Hermes learning integration go/no-go** | 调研 ✅ | 代码 ❌
  6 篇规划文档（全部 draft, 2026-04-05），13-18 周路线图，零进展。
  - Source: `docs/dev/awesomeagents/hermes-learning/` + `hermes-integration/`

- [ ] **D-2: HA'S-PROJECT memory system replacement** | 调研 ✅
  HA'S-PROJECT vs Hermes memory 冲突解决方案，阻塞 Hermes 集成。
  当前实现：LAPUTA + MENTLE 已覆盖记忆需求，HA'S-PROJECT 无必要（拉黑原因：叙述过于民科气质）。

- [ ] **D-3: SQLite vs file-backed JSON for plan storage** | 调研 ✅
  pre-implementation-research 推荐 SQLite (sqlx)，sqlx 已可用。需在 PR #1 前确认。

- [ ] **D-4: 5-layer bypass prevention design review** | 调研 ✅ | 代码 ❌
  Spec 定义的 5 层防护全部未实现 (filesystem intercept → tool layer gate → PlanOrchestrator → audit log → context injection)。依赖 Plan Mode (P1-1)。

- [ ] **D-5: NAG mechanism threshold validation** | 调研 ✅ | 代码 ❌
  3 轮无 planning tool → 注入提醒。全代码库无 nag/reminder/inject 实现。依赖 Plan Mode。

### Housekeeping

- [ ] **H-1: Broken link in docs/dev/README.md** — 引用了不存在的 `nano-runtime-packaging-plan.md`

- [ ] **H-2: awesomeagents/decisions.md uncommitted changes** — git status 显示有工作区修改

- [ ] **H-3: Self-evolution UI research tag** — `genericagent/newedge/agent-diva-pro-self-evolution-ui-research.md` 标记为 "future/pro"

- [ ] **H-4: plan-todo-ui-scope-extract.md completeness** — 验证提取摘要与源文档一致性

---

## Implementation Dependency Graph

```
H-1, H-2, H-3, H-4 ──→ independent (docs only)

P0-3 (credential scrubbing) ──→ independent, quick win (add tracing Layer + regex feature)
P0-6 (context overflow) ──→ independent
P1-2 (observability) ──→ independent, can proceed now
P1-5 (tool timeout) ──→ independent, quick win (add timeout in ToolRegistry)
P1-6 (error classification) ──→ independent

P0-4 (session truth-source) ──→ prerequisite for P1-1
D-3 (SQLite) ──→ P1-1 (Plan+TodoList) ──→ D-4 (bypass prevention) + D-5 (NAG) + P1-4 (permission UI)
P0-1 (circuit breaker) + P0-2 (subagent security) + P1-3 (sandbox) ──→ overlapping security work, can batch

D-1 (Hermes learning) ──→ long-term decision chain
```

---

## Done

- [x] Improve GUI image input experience for multimodal vision. (2026-06)
- [x] Path traversal hardening — 8-layer validation implemented. (pre-2026-06, TODOLIST was stale)
- [x] 2026-06-03 docs/dev comprehensive survey (37 active files → 17 TODO items) + code audit (15 items checked)
  - Survey outputs: `docs/dev/_survey_awesomeagents.md`, `docs/dev/_survey_other.txt`
  - Audit outputs: `docs/dev/_audit_p0.md`, `docs/dev/_audit_p1.md`
  - Total sub-agent cost: ~$4.13 (4 agents)
