# TODOLIST

## Main Closeout (2026-06)

- [ ] Close the mixed `main` working tree using the repo-local closeout docs instead of one bulk commit.
- [ ] Follow [docs/dev/main-closeout-plan-2026-06.md](./docs/dev/main-closeout-plan-2026-06.md) as the authoritative closeout rule set.
- [ ] Execute [docs/dev/main-closeout-cards-2026-06.md](./docs/dev/main-closeout-cards-2026-06.md) in order, one clean theme at a time.
- [ ] Keep frontend/product files out of `main` closeout unless they are explicitly re-scoped later.

This file is the project-level backlog for bugs, gaps, and unfinished work found during development or review.

Last comprehensive scan: 2026-06-03 (`docs/dev` survey — 37 active .md files)
Last code audit: 2026-06-03 (2x Claude Code, 15 items, total cost ~$4.13)

Legend: 调研 ✅=已完成  🔄=进行中  ❌=未开始 | 代码 ✅=已实现  🟡=部分  ❌=未实现

---

## Open

### P0 — Security & Stability

- [x] **P0-1: Infinite loop / circuit breaker** | 调研 ✅ | 代码 ✅ | **2026-06-04 已完成**
  Agent 在工具重复失败时可能无限循环，缺少工具调用 hash 去重和迭代预算。
  - 现存: `max_iterations=20` (agent_loop.rs:100), subagent 硬编码 15 (subagent.rs:252)
  - 缺失: 无 circuit_breaker 模块/struct, 无 tool-call hash 去重, 无连续失败检测, 无 wall-clock timeout, 无 token/cost budget
  - Source: `docs/dev/awesomeagents/unknown-deficits.md` (Defect 1), `docs/dev/awesomeagents/decisions.md` (P0-2)

- [x] **P0-2: Sub-agent security suite** | 调研 ✅ | 代码 ✅ | **2026-06-04 已完成**
  缺少子代理安全控制：深度限制、凭据最小化、并发控制。
  - 现存: 硬编码 tool blacklist (for_subagent() 禁用 spawn/cron/attachment), SecurityPolicy 8 层路径校验
  - 缺失: 无 max_depth, 无 subagent 并发上限 (SubagentManager 只 track 不限流), 无 credential minimization (subagent 继承完整 API key + network config + MCP servers)
  - Source: `docs/dev/awesomeagents/decisions.md` (P0-1), `docs/dev/awesomeagents/sandbox-audit-c.md`

- [x] **~~P0-3: Credential scrubbing in logs~~** | 调研 ✅ | 代码 ✅ — **2026-06-04 已实现**
  已新增 `agent-diva-core::redaction`，并在 `logging.rs` 使用 redacting writer 对 stdout/file tracing 输出做统一脱敏，同时为 `ErrorContext` 与 manager `ConfigUpdate` 日志摘要补充保护。
  - 已覆盖: `Bearer ...`, `sk-*`, `ghp_*`, `xoxb-*`, 以及 `api_key` / `token` / `secret` / `password` / `authorization` 字段
  - 验证: `just fmt-check` ✅, `just check` ✅, `cargo test -p agent-diva-core redaction/logging/error_context` ✅, `cargo test -p agent-diva-cli config_show_json_redacts_secrets` ✅
  - Source: `docs/dev/awesomeagents/sandbox-audit-b.md`, `docs/logs/2026-06-log-redaction/v0.0.1-p0-3-credential-scrubbing/`

- [x] **~~P0-4: Session truth-source fix (Phase A-PRE)~~** | 调研 ✅ | 代码 ✅ — **2026-06-04 已完成**
  后端 durability 与 GUI truth-source/backend-first reconciliation 已完成闭环。
  - 已修复: inbound user message 在 LLM/tool 执行前立即写入 session 并持久化
  - 已修复: raw turn 先 durable save，再运行 consolidation，再持久化 `last_consolidated`
  - 已修复: `SessionManager::save()` 改为 temp file + backup promote，`load()` 读失败/解析失败不再静默当作新 session
  - 已修复: GUI `loadSession()` backend-first、cache fallback 提示、send/reset/delete/switch/stop 的 canonical reconciliation
  - Source: `docs/dev/agent-plan/phase-a-pre-session-truth-source-fix.md`, `docs/logs/2026-06-session-truth-source/v0.0.1-p0-4-backend-durability/`, `docs/logs/2026-06-session-truth-source/v0.0.2-p0-4-frontend-reconciliation/`

- [x] **~~P0-5: Path traversal hardening~~** | 调研 ✅ | 代码 ✅ — **已实现，TODOLIST 过时**
  已全面实现 8 层路径校验 (security/path.rs): null bytes → ParentDir → URL-encoded traversal → tilde → absolute → forbidden prefix → canonicalize → symlink escape。文件工具 + shell + skill_zip 均有调用。测试覆盖: test_path_traversal_blocked, upload_skill_zip_rejects_path_traversal。

- [x] **P0-6: Context overflow silent truncation** | 调研 ✅ | 代码 ✅ | **2026-06-04 已完成**
  新增 agent-level context budget 与 overflow recovery，消除当前仅靠字符截断和 provider 400 兜底的静默退化。
  - 已实现: `agents.defaults.context_budget_tokens` / `context_budget_reserve_tokens` / `context_overflow_retry_enabled`
  - 已实现: 启发式 token 估算、请求前 context 裁剪、单次 overflow 恢复重试、明确用户文案
  - 已覆盖: main agent + subagent 调用路径，不引入 tokenizer 或 LLM summary compaction
  - Source: `docs/dev/awesomeagents/unknown-deficits.md` (Defect 2), `docs/logs/2026-06-agent-loop-safety/v0.0.3-p0-6-context-overflow-guardrail/`

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

- [x] **~~P1-5: Tool execution timeout wrapping~~** | 调研 ✅ | 代码 ✅ — **2026-06-04 已实现**
  已在 `agent-diva-tooling::ToolRegistry::execute()` 增加统一 `tokio::time::timeout` 包裹，并将 `tools.exec.timeout` 复用为 registry-level 默认工具超时。
  - 已实现: registry 默认 60s 超时、统一 timeout 错误包装、`ToolAssembly` 将 `exec_timeout` 下发到 registry
  - 保留: Shell 与 MCP 工具内部已有 timeout；registry timeout 作为总兜底，不替代细粒度超时
  - 校验: `tools.exec.timeout` 现在要求 `> 0`
  - 验证: `just fmt-check` ✅, `just check` ✅, `cargo test -p agent-diva-tooling registry --lib` ✅, `cargo test -p agent-diva-agent tool_assembly --lib` ✅, `cargo test -p agent-diva-core validate --lib` ✅
  - 注意: `just test` 仍受既有 `H-5` 阻塞，失败点为 `agent-diva-agent::skills::tests::test_default_builtin_dir_loads_skills`

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

- [x] **H-5: agent-diva-agent builtin skill smoke test failing in local validation** | **2026-06-04 已修复**
  默认 builtin skill 发现已改为优先解析真实可用目录，`just test` 已恢复全绿。
  - Fix: `agent-diva-agent/src/skills.rs`
  - Validation: `just fmt-check`, `just check`, `just test`

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

- [x] P0-1 infinite loop / circuit breaker closed. (2026-06-04)
  - Added shared `agent-diva-agent::loop_guard` for main agent loop and subagent loop.
  - Added repeated identical tool-failure breaker, stable tool-call fingerprinting, and loop wall-clock timeout.
  - Iteration notes: `docs/logs/2026-06-agent-loop-safety/v0.0.1-p0-1-circuit-breaker/`
- [x] P0-2 sub-agent security suite closed. (2026-06-04)
  - Added `tools.subagent` least-privilege defaults, concurrency limit, depth limit, and subagent policy-based tool rebuilding.
  - Subagent web search credentials are stripped by default, web fetch is disabled by default, and MCP is disabled by default.
  - Iteration notes: `docs/logs/2026-06-agent-loop-safety/v0.0.2-p0-2-subagent-security-suite/`
- [x] P0-6 context overflow guardrail closed. (2026-06-04)
  - Added heuristic context budget estimation, proactive compaction, overflow classification, and one retry with stronger trimming.
  - Added agent config defaults for context budget and reused the same guardrail in subagent execution.
  - Iteration notes: `docs/logs/2026-06-agent-loop-safety/v0.0.3-p0-6-context-overflow-guardrail/`
- [x] Improve GUI image input experience for multimodal vision. (2026-06)
- [x] P1-5 tool execution timeout wrapping. (2026-06-04)
- [x] P0-4 session truth-source fix (backend durability + GUI reconciliation). (2026-06-04)
- [x] P0-3 credential scrubbing in logs. (2026-06-04)
- [x] Path traversal hardening — 8-layer validation implemented. (pre-2026-06, TODOLIST was stale)
- [x] 2026-06-03 docs/dev comprehensive survey (37 active files → 17 TODO items) + code audit (15 items checked)
  - Survey outputs: `docs/dev/_survey_awesomeagents.md`, `docs/dev/_survey_other.txt`
  - Audit outputs: `docs/dev/_audit_p0.md`, `docs/dev/_audit_p1.md`
  - Total sub-agent cost: ~$4.13 (4 agents)
