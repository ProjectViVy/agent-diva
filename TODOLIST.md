# TODOLIST

## Main Closeout (2026-06)

- [x] Close the mixed `main` working tree using the repo-local closeout docs instead of one bulk commit. **2026-06-06 completed via MAIN-CLOSE-01..05 commits.**
- [x] Follow [docs/dev/main-closeout-plan-2026-06.md](./docs/dev/main-closeout-plan-2026-06.md) as the authoritative closeout rule set. **done**
- [x] Execute [docs/dev/main-closeout-cards-2026-06.md](./docs/dev/main-closeout-cards-2026-06.md) in order, one clean theme at a time. **done**
- [x] Keep frontend/product files out of `main` closeout unless they are explicitly re-scoped later. **done — marked as moved-out in closeout cards**

This file is the project-level backlog for bugs, gaps, and unfinished work found during development or review.

Last comprehensive scan: 2026-06-03 (`docs/dev` survey — 37 active .md files)
Last code audit: 2026-06-03 (2x Claude Code, 15 items, total cost ~$4.13)
Last routing review: 2026-06-07 (`main` retains stability-only work; `context-compaction` confirmed complete on `agent-diva-pro/feature/context-compaction`)

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

- [ ] **P1-2: Phase B: Thin Observability Layer** | 调研 ✅ | 代码 🟡
  Design complete, blocks on Phase A。Tracing 基础设施完善但 spec 合规度低。
  - 现存: tracing-subscriber (EnvFilter + rolling file), trace_id (Uuid), 12 structured trace points (loop_turn.rs)
  - 缺失: 无 typed TraceId/TraceEvent, 无 JSONL writer, 无 redaction layer, 无 structured event emission, 无 debug bundle

- [ ] **P1-3: Sandbox audit remediation (route TBD)** | 调研 ✅ | 代码 🟡
  3 份审计报告 (A/B/C) ~20 发现待修复。注意: `agent-diva-sandbox` crate 不在当前 workspace。
  当前路线说明: sandbox 暂不视为必然回流 `main`；现阶段优先在 `pro` 分支继续验证，待实验结果稳定后再决定是否抽取 backend/runtime 安全能力回流主线，或仅保留为独立实验线。
  - 已实现 (core security): SecurityPolicy 8-layer, PathValidator, SecurityConfig levels, ActionTracker rate limiter, shell deny patterns, forbidden paths/extensions
  - 缺失: platform-level sandbox (RestrictedToken/Landlock/Seatbelt), env filtering, prompt injection scanning, MCP limits, subagent concurrency

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

### Housekeeping

- [ ] **H-1: Broken link in docs/dev/README.md** — 引用了不存在的 `nano-runtime-packaging-plan.md`

- [x] **H-5: agent-diva-agent builtin skill smoke test failing in local validation** | **2026-06-04 已修复**
  默认 builtin skill 发现已改为优先解析真实可用目录，`just test` 已恢复全绿。
  - Fix: `agent-diva-agent/src/skills.rs`
  - Validation: `just fmt-check`, `just check`, `just test`

### Moved Out / Archived From `main` (2026-06-07 routing review)

- [x] **P1-1: Plan+TodoList implementation** — 从 `main` 当前 backlog 归档
  该项属于新能力/流程模式，不属于“主分支只做稳固性提升”的当前边界。待后续被重新定义为独立 backend epic 后再重新开卡。
  - Source: `docs/dev/agent-plan/`

- [x] **P1-4: Permission mode UI wired to backend** — 移出 `main`
  该项是明显的 product/UI + backend 协同主题，不应继续挂在 `main` 稳固性 backlog 下。后续如保留，应拆成 backend contract 与 `pro` UI 接线两张卡。
  - Source: `docs/dev/awesomeagents/pro-ui-audit.md`

- [x] **D-1: Hermes learning integration go/no-go** — 归档到研究线
  属于 `selfinprove` / 研究决策，不是 `main` 当前稳定线待办。

- [x] **D-2: HA'S-PROJECT memory system replacement** — 归档到研究线
  属于长期记忆架构路线判断，不是 `main` 当前稳定线待办。

- [x] **D-3: SQLite vs file-backed JSON for plan storage** — 连同 Plan Mode 一并归档
  该决策只服务于 `P1-1`，在 Plan+TodoList 未重新纳入主线前不再保留为 `main` 开放项。

- [x] **D-4: 5-layer bypass prevention design review** — 归档，等待 Plan Mode 重新立项
  依赖 `P1-1`，当前不属于 `main` 稳定线的直接工作。

- [x] **D-5: NAG mechanism threshold validation** — 归档，等待 Plan Mode 重新立项
  依赖 `P1-1`，当前不属于 `main` 稳定线的直接工作。

- [x] **H-2: awesomeagents/decisions.md uncommitted changes** — 关闭为过时项
  2026-06-07 复核时 `git status` 已干净，此项不再成立。

- [x] **H-3: Self-evolution UI research tag** — 移出 `main`
  该项属于 `pro` / 研究线文档整理，不属于 `main` 稳定性范围。

- [x] **H-4: plan-todo-ui-scope-extract.md completeness** — 移出 `main`
  该项服务于 Plan/Todo UI 主题，不属于 `main` 当前稳定性范围。

---

## Implementation Dependency Graph

```
H-1 ──→ independent (docs only)

P1-2 (observability) ──→ independent, can proceed on `main`
P1-6 (error classification) ──→ independent, can proceed on `main`
P1-3 (sandbox remediation, route TBD) ──→ validate on `pro` first; only batch into `main` if backend/runtime-safe slices are later approved to return

Plan/permission/research decisions were moved out of the active `main` backlog on 2026-06-07.
```

---

## Done

- [x] H-1 docs/dev README broken nano link fixed. (2026-06-07)
  - Replaced the dead `nano-runtime-packaging-plan.md` link with the archived nano/shared-runtime packaging index.
- [x] P1-2 thin observability minimum slice landed. (2026-06-07)
  - Added `agent-diva-core::trace` with typed `TraceId`, `TraceEvent`, JSONL writer, redaction, truncation, and retention-aware cleanup.
  - Added `logging.structured_runtime_logs_enabled`, `logging.retention_days`, `logging.runtime_log_dir`, and `logging.record_tool_output_summaries`.
  - Agent runtime now emits structured `message_received`, `llm_request_started`, `llm_response_completed`, `llm_response_failed`, `tool_call_started`, `tool_call_completed`, `tool_call_failed`, and `runtime_cancelled`.
  - Remaining observability backlog stays open for debug bundle export, gateway/channel events, and GUI settings.
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
- [x] Context compaction ownership moved to `agent-diva-pro`, and the corresponding line is no longer an open `main` backlog item. (2026-06-07)
  - Reference: `../MOREDIVA-context-compaction-handoff-2026-06-07.md`
- [x] Improve GUI image input experience for multimodal vision. (2026-06)
- [x] P1-5 tool execution timeout wrapping. (2026-06-04)
- [x] P0-4 session truth-source fix (backend durability + GUI reconciliation). (2026-06-04)
- [x] P0-3 credential scrubbing in logs. (2026-06-04)
- [x] Path traversal hardening — 8-layer validation implemented. (pre-2026-06, TODOLIST was stale)
- [x] 2026-06-03 docs/dev comprehensive survey (37 active files → 17 TODO items) + code audit (15 items checked)
  - Survey outputs: `docs/dev/_survey_awesomeagents.md`, `docs/dev/_survey_other.txt`
  - Audit outputs: `docs/dev/_audit_p0.md`, `docs/dev/_audit_p1.md`
  - Total sub-agent cost: ~$4.13 (4 agents)
