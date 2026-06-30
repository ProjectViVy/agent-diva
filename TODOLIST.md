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

## Bug Ledger

This section is the dedicated bug-only board. It tracks confirmed defects only, separate from feature work, architecture work, docs cleanup, and general backlog items.

### Audit Batch: 2026-06-23 BMad / Harness Cross-Reference (22 bugs)

Source:
- `docs/research/evidence/cross-cutting/diva-harness-cross-reference-audit.md`
- `docs/research/evidence/self-audits/diva-security-sandbox-self-audit.md`

Current sync status on 2026-06-30 (re-audited against feat/harness-wave0):
- Total confirmed bugs in this batch: `22`
- Closed: `19` (B-01~B-17, B-20~B-22)
- Open: `0`
- Feature Backlog: `2` (B-18, B-19)
- 2026-06-30 feat/harness-wave0 re-audit: B-03 (structured audit in commit `3c44f0e`), B-07 (guardian tightened in commit `a473a57`), B-09 (approval cache unified in commit `47f3195`) are implemented on `feat/harness-wave0`. Remaining 4 open bugs are genuine code defects in `agent-diva-core/src/security/` and `agent-diva-core/src/bus/`.

- [x] `B-01` Provider token usage discarded
  - Status: fixed
  - Notes: main agent now emits `TokenUsed` events and audit events; subagent accumulates `token_usage`.
  - Related: `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-core/src/audit/audit.rs`, `agent-diva-agent/src/subagent.rs`

- [x] `B-02` System prompt budget reserved, not actually measured
  - Status: fixed
  - Notes: system prompt token cost is now measured from the rendered first system message and subtracted from the usable history/tool budget before compaction/truncation on both main-agent and subagent paths.
  - Related: `agent-diva-agent/src/context_budget.rs`

- [x] `B-03` No persistent security audit trail
  - Status: fixed
  - Notes: structured audit logging implemented via `agent-diva-core/src/audit/` with JSONL sink and GUI audit page. Commit `3c44f0e` on `feat/harness-wave0`.
  - Related: `agent-diva-core/src/audit/audit.rs`, `agent-diva-core/src/logging.rs`

- [x] `B-04` Consolidation has no quality gate
  - Status: fixed
  - Notes: consolidation now extracts expected keywords from the source segment, applies a quality gate with bounded regeneration retries, rejects low-quality output, and only advances `last_consolidated` under the explicit valid-tool-call policy.
  - Related: `agent-diva-agent/src/summary_compaction.rs`

- [x] `B-05` No general tool timeout
  - Status: fixed
  - Notes: registry-wide timeout wrapper exists; effective timeout is now `registry default` or `tool explicit override`.
  - Related: `agent-diva-tooling/src/base.rs`, `agent-diva-tooling/src/registry.rs`

- [x] `B-06` Rate limiter is global, not per-session/per-user
  - Status: fixed
  - Expected: rate limiting should be keyed, not process-global.
  - Related: `agent-diva-core/src/security/rate_limit.rs`, `agent-diva-core/src/security/policy.rs`

- [x] `B-07` Guardian default is overly conservative
  - Status: fixed
  - Notes: guardian default approvals tightened in sandbox audit remediation batch. Commit `a473a57` on `feat/harness-wave0`.
  - Related: `agent-diva-core/src/security/policy.rs`

- [x] `B-08` No compaction-of-compaction
  - Status: fixed
  - Notes: accepted summaries now accumulate in a live `SummaryChain`, trigger bounded meta-compaction after the configured threshold, retain `source_summary_ids`, and respect max-depth limits.
  - Related: `agent-diva-agent/src/summary_compaction.rs`

- [x] `B-09` Duplicate approval types not merged
  - Status: fixed
  - Notes: approval cache access unified in sandbox refactor batch. Commit `47f3195` on `feat/harness-wave0`.
  - Related: `agent-diva-core/src/security/`

- [x] `B-10` Windows file locking missing in exec policy persistence
  - Status: **deferred**
  - Notes: exec policy persistence belongs to `agent-diva-sandbox` crate (experimental branch). Not present in current workspace.

- [x] `B-11` URL double-encoding bypass (`%252f`, `%255c`)
  - Status: fixed
  - Expected: path validation should reject double-encoded traversal too.
  - Related: `agent-diva-core/src/security/path.rs`, `agent-diva-core/src/security/policy.rs`

- [x] `B-12` Tool error detection via string prefix
  - Status: fixed
  - Notes: structured `ToolError` flow is now used for registry execution; `patch` and `search_files` no longer report pseudo-success error strings.
  - Related: `agent-diva-tooling/src/registry.rs`, `agent-diva-tools/src/patch.rs`, `agent-diva-tools/src/search_files.rs`

- [x] `B-13` NagTracker wiring unclear
  - Status: **false positive** — moved to Feature Backlog
  - Notes: `NagTracker` / `PlanOrchestrator` / `approved_plans` were designed in May 2026 plan-mode research but **never implemented** in Rust source. The audit document `diva-planning-budget-self-audit.md` incorrectly described them as existing code. No code to fix.
  - Resolution: Plan mode feature not built; re-scope as separate epic when plan mode is prioritized.

- [x] `B-14` HOOK-3 / HOOK-4 are no-ops
  - Status: **false positive** — moved to Feature Backlog
  - Notes: Same as B-13. These hooks are stubs for a plan mode that was never implemented. Not code bugs.
  - Resolution: Same as B-13.

- [x] `B-15` Approval state not persisted
  - Status: **false positive** — moved to Feature Backlog
  - Notes: `PlanOrchestrator::approved_plans` was never built. The in-memory `HashSet` described in the audit does not exist in any Rust source file.
  - Resolution: Same as B-13.

- [x] `B-16` Unbounded event bus channels
  - Status: fixed
  - Expected: introduce bounded/backpressure-aware behavior or overflow strategy.
  - Related: `agent-diva-core/src/bus/queue.rs`

- [x] `B-17` Compaction prompt is Chinese-only
  - Status: fixed
  - Notes: compaction and meta-compaction prompts now support `auto` / `en` / `zh`, with `auto` as the default language-aware path for mixed-language sessions.
  - Related: `agent-diva-agent/src/summary_compaction.rs`

- [ ] `B-18` No system prompt caching
  - Status: **moved to Feature Backlog**
  - Notes: this is a feature enhancement (provider caching layer), not a code defect. Research-only for now. See PM plan in Open section.
  - Related: agent loop audit findings

- [ ] `B-19` No plugin/middleware hooks around LLM calls
  - Status: **moved to Feature Backlog**
  - Notes: `agent-diva-hooks` crate was implemented (v0.1.0, ~1200 LOC) but reverted on 2026-06-30 — not reviewed, not in plan. Re-scope as BMAD epic when prioritized.
  - Related: agent loop audit findings, `agent-diva-hooks/` (reverted)

- [x] `B-20` Hardcoded LLM params (`temperature=0.7`, `max_tokens=4096`)
  - Status: fixed
  - Notes: maintenance/helper calls now use dedicated `agents.defaults.context_maintenance` settings for `max_tokens`, `temperature`, quality thresholds/retries, meta-compaction limits, and prompt language mode instead of hardcoded helper values.
  - Related: `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/src/subagent.rs`

- [x] `B-21` Keyword extraction is simplistic
  - Status: fixed
  - Notes: summary/consolidation quality checks now share normalized keyword extraction with ASCII token handling, bounded CJK chunk extraction, punctuation cleanup, and deduplication.
  - Related: planning audit findings

- [x] `B-22` Heartbeat has no retry/backoff
  - Status: fixed
  - Notes: heartbeat decide calls now share bounded retry/backoff logic across `trigger_now()` and background ticks; exhausted retries emit explicit `error` heartbeat outcomes and skip execute for that tick.
  - Related: `agent-diva-core/src/heartbeat/service.rs`, `agent-diva-core/src/heartbeat/types.rs`

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

### P0 — Harness v1.1 Ultimate Research (PRD v1.1 前置)

目标：在重写 Harness Engineering PRD v1.1 之前，完成对 Provider、Channel、Cron、Skill、Agent loop 可持续性、Config migration 六大领域的深度审计，确保「终极 Harness 增强」没有盲区。

- [x] **P0-R1: Provider 全链路深度审计** | 调研 ✅ | 代码 ❌ | **2026-06-26 已完成**
  为 Harness v1.1 提供 Provider 层完整现状与缺口清单，支撑 Epic 5 Provider 修复和 TokenUsed 事件设计。
  - 输出：`docs/research/diva-providers-full-audit-v2.md`
  - 下游：PRD v1.1 Epic 5-A

- [x] **P0-R2: Channel 全链路深度审计** | 调研 ✅ | 代码 ❌ | **2026-06-26 已完成**
  为 Harness v1.1 提供 Channel 层完整缺口清单，支撑安全与审计设计。
  - 输出：`docs/research/diva-channels-full-audit-v2.md`
  - 下游：PRD v1.1 Epic X

- [x] **P0-R3: Cron 全链路深度审计** | 调研 ✅ | 代码 ❌ | **2026-06-26 已完成**
  为 Harness v1.1 提供 Cron 层完整缺口清单，支撑 CronService 接入 Module trait。
  - 输出：`docs/research/diva-cron-full-audit-v2.md`
  - 下游：PRD v1.1 Epic 4 Cron 部分

- [x] **P0-R4: Skill 系统深度审计** | 调研 ✅ | 代码 ❌ | **2026-06-26 已完成**
  为 Harness v1.1 提供 Skill 层完整缺口清单，支撑安全与上下文质量设计。
  - 输出：`docs/research/diva-skills-full-audit-v2.md`
  - 下游：PRD v1.1 Epic 2

- [x] **P0-R5: Agent Loop 可持续性审计** | 调研 ✅ | 代码 ❌ | **2026-06-26 已完成**
  为 Harness v1.1 提供主 agent loop 在长会话、高负载下的可持续性分析。
  - 输出：`docs/research/diva-agent-loop-full-audit-v2.md`
  - 下游：PRD v1.1 Epic 7 + Epic 5-B

- [x] **P0-R6: Config Migration / Versioning 审计** | 调研 ✅ | 代码 ❌ | **2026-06-26 已完成**
  为 Harness v1.1 提供配置 schema 演进策略，支撑 Epic 4 热重载和老用户升级。
  - 输出：`docs/research/diva-config-migration-full-audit-v2.md`
  - 下游：PRD v1.1 Epic 4 Config 部分

- [x] **P0-R7: Harness Engineering PRD v1.1 定稿** | 调研 ✅ | 代码 ❌ | **2026-06-26 终稿已完成**
  基于 P0-R1~R6 审计结论和最终验收报告，完成 Harness Engineering PRD v1.1 终稿并归档旧 PRD。
  - 输出：`docs/prds/prd-harness-engineering-v1.1/prd.md`
  - 旧 PRD 归档：`docs/prds/archive/prd-harness-engineering-2026-06-24/prd.md`（已移动 + README 索引）
  - B-8 核查结果写入 v1.1 §4.3：Module 生命周期与 Presence 状态机已实现；Audit/PII/Injection 部分实现；新增 `instruction_hierarchy.rs` 与 `tool_result_filter.rs` 作为 Epic 2 增量 story
  - 已闭合：PC-1（Batch 2 REVISE）、PC-3（reference path）

---

---

### Plan Mode — Feature Backlog (2026-06-30)

> Plan Mode（agent 先计划后执行，含 plan 文件生成、只读工具限制、上下文压缩/清理、计划审批流程）于 2026-05 完成设计但**从未实现为 Rust 代码**。B-13/B-14/B-15 审计时误判为已有代码缺陷，实为未实现功能。

- [x] **PM-0: 调研 — Plan Mode 设计现状与缺口分析** | **2026-06-30 已完成**
  - 三批次调研（oh-my-pi、claude-code、codex、OpenHarness、pi、openfang）+ diva 代码级 gap matrix
  - 结论：`PlanOrchestrator` / `NagTracker` / `approved_plans` **未实现**；Hybrid 八层栈（ExitPlanMode + OMP guard + OH checker + pi MVP）
  - 交付物目录：[docs/research/plan-mode/](./docs/research/plan-mode/)
    - PM-0 终稿：[plan-mode-gap-analysis-final.md](./docs/research/plan-mode/plan-mode-gap-analysis-final.md)
    - 目标架构 + P0 stories：[diva-plan-mode-target-architecture.md](./docs/research/plan-mode/diva-plan-mode-target-architecture.md)
    - 索引：[README.md](./docs/research/plan-mode/README.md)
  - Source: `docs/research/plan-mode/`, `agent-diva-core/src/security/policy.rs`, 参考代码 `agent-diva/.workspace/`

- [ ] **PM-1: 新建 Epic — 走完整 BMAD 工作流**
  1. `/bmad-prd` — 基于调研结论撰写 Plan Mode PRD
  2. `/bmad-architecture` — Plan Mode 架构 spine
  3. `/bmad-check-implementation-readiness` — 验证 PRD+UX+Architecture 完整
  4. `/bmad-create-epics-and-stories` — 分解为 Epics & Stories
  5. `/bmad-sprint-planning` — 生成 sprint 计划
  - 输出到: `_bmad-output/planning-artifacts/`
  - **前置依赖**: PM-0 调研完成

### P1 — Core Infrastructure

- [ ] **P1-2: Phase B: Thin Observability Layer** | 调研 ✅ | 代码 🟡
  Design complete, blocks on Phase A。Tracing 基础设施完善但 spec 合规度低。
  - 现存: tracing-subscriber (EnvFilter + rolling file), trace_id (Uuid), 12 structured trace points (loop_turn.rs)
  - 缺失: 无 typed TraceId/TraceEvent, 无 JSONL writer, 无 redaction layer, 无 structured event emission, 无 debug bundle
  - 2026-06-08 update: `gateway run --debug` and `gateway bundle` added for explicit raw debug runs.
  - Remaining gap: debug mode records full payloads visible at agent/runtime boundaries, but does not yet tap provider-native final HTTP request/response bytes or MCP SDK internal RPC frames. Expected behavior: deeper raw taps must remain gated behind explicit debug mode and be included in the same debug run bundle. Related files: `agent-diva-providers/src/litellm.rs`, `agent-diva-tools/src/mcp_sdk.rs`, `agent-diva-core/src/debug.rs`.

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

 - [x] **P1-7: Wire heartbeat cadence to PresenceState** | fixed | **2026-06-30 completed**
   Heartbeat cadence now derives its effective interval from current `PresenceState` plus configured multipliers instead of fixed rhythm constants.
   - Implemented: `Active` uses base interval, `Distracted` and `Gone` share `presence.distracted_heartbeat_multiplier`, `Away` remains suspended and only re-checks later.
   - Implemented: cadence computation clamps to a minimum effective interval of 1 second and is covered by heartbeat/presence/config tests.
   - Related: `agent-diva-core/src/heartbeat/service.rs`, `agent-diva-core/src/presence/state_machine.rs`, `agent-diva-core/src/config/reload_plan.rs`

- [ ] **P1-8: Provider architecture simplification** | 调研 ✅ | 代码 ❌
  将 provider 层从 13 槽位 + 47 YAML 收敛为仅保留 Anthropic 原生 + OpenAI-compatible 两条链路，其他 provider 全部通过用户自部署转接层接入。**质量要求：生产级完整**，支持 retry/fallback/rate-limit/token usage/tool schema/完整错误分类。
  - 决策文档：`docs/dev/provider-simplification-research-2026-06.md`
  - 目标：删除 `providers.yaml` 中多余条目、精简 `ProvidersConfig`、新增 `AnthropicDriver`、强化 `OpenAiCompatibleDriver`、补充 retry/fallback 中间层
  - 相关文件：`agent-diva-providers/src/litellm.rs`, `agent-diva-providers/src/base.rs`, `agent-diva-providers/src/registry.rs`, `agent-diva-providers/src/providers.yaml`, `agent-diva-core/src/config/schema.rs`, `agent-diva-manager/src/runtime.rs`

- [ ] **P1-9: Channel architecture simplification** | 调研 ✅ | 代码 ❌
  将 channel 层从 13 个硬编码 adapter 收敛为 8 个一等公民 + Matrix + Neuro-Link，其余移除或未来插件化。**质量要求：生产级完整**，每个保留 channel 必须支持群聊/频道/私聊、文件/媒体收发、完整入站/出站链路、无 OAuth 配置方式。
  - 决策文档：`docs/dev/channel-simplification-decision-2026-06.md`
  - 一等公民：Telegram、Discord、Slack、Email、QQ、Feishu/Lark、DingTalk、WeChat（新增）
  - 保留：Matrix（开源联邦，未来价值）、Neuro-Link（ interim 通用入口，未来重构）
  - 移除/插件化：WhatsApp、Mattermost、Nextcloud Talk、IRC
  - 不做：OAuth/网页登录/云平台 IAM channel、社交/内容平台
  - 目标：新增 WeChat adapter、移除 4 个 deprecated channel、引入 per-channel feature flag、精简 `ChannelsConfig`、全面增强保留 channel、更新 README/GUI/用户文档
  - 相关文件：`agent-diva-channels/src/*.rs`, `agent-diva-core/src/config/schema.rs`, `README.md`, `agent-diva-gui/`

### Housekeeping

- [ ] **H-8: Workspace-wide `just check` / `just test` baseline is still red outside this batch**
  2026-06-30 validation for the heartbeat batch confirmed this change set, but full workspace gates still fail for unrelated existing issues.
  - `just check`: pre-existing clippy-denied warnings/errors in `agent-diva-tooling/src/registry.rs` and `agent-diva-providers/src/{anthropic,dto.rs,litellm/client.rs,litellm/dto.rs}`.
  - `just test`: pre-existing failure in `agent-diva-migration/src/config_migration.rs` still asserts removed `providers.openai` schema.
  - Expected: restore workspace-wide green CI so focused fixes can rely on `just ci` again.

- [ ] **H-7: Audit frontend dependency vulnerability sweep**
  `npm ci` on 2026-06-25 reported 10 frontend dependency vulnerabilities (6 moderate, 4 high) under `agent-diva-gui`.
  - Expected: audit the reported packages, decide whether upgrades are safe, and capture any required compatibility work before the next GUI delivery.
  - Related: `agent-diva-gui/package.json`, `agent-diva-gui/package-lock.json`

- [ ] **H-1: Broken link in docs/dev/README.md** — 引用了不存在的 `nano-runtime-packaging-plan.md`

- [x] **H-6: all-targets clippy cleanup in core tests** — **2026-06-11 已修复**
  全部 8 个 clippy 错误已机械修复：`agent-diva-core/src/session/manager.rs` 5 处 `needless_borrow` + 1 处 `unnecessary_get_then_check`；`agent-diva-core/src/soul/mod.rs` 1 处 `field_reassign_with_default`。
  - 验证: `cargo clippy -p agent-diva-core --all-targets -- -D warnings` 退出码 0
  - Commit: `1e33a73 fix(core): clean up all-targets clippy warnings`

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
- [x] Builtin tool toggle drift and delegation semantics drift closed. (2026-06-30)
  - `tools.builtin.search_files`, `code_execution`, and `delegate` now map 1:1 from config through CLI/manager runtime assembly instead of being partially hardcoded.
  - `ToolAssembly` now treats `filesystem` and `search_files` as separate capabilities, gates `execute_code` independently, and only registers delegation when `delegate && spawn` and not in subagent mode.
  - Subagent runtime continues to force-disable delegation and code execution regardless of parent toggles.
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
