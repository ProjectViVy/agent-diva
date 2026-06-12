# Survey: docs/dev/awesomeagents/

> Survey date: 2026-06-03
> Total files found: 24 (all verified to exist and be readable)

---

## Active files found (non-archive)

### 1. `hermes-integration/00-current-architecture-analysis.md`
- **Title:** Agent-Diva 现有架构分析报告
- **Summary:** Deep analysis of agent-diva's current architecture, identifying integration points for Hermes self-learning mechanisms. Maps workspace crate structure, message bus, memory system, and agent loop extension points.
- **Status:** Research/analysis — completed reference document.

### 2. `hermes-learning/00-executive-summary.md`
- **Title:** Hermes 自我学习机制集成规划 - 执行摘要
- **Summary:** Executive summary for integrating Hermes' self-learning capabilities (RL training via Tinker-Atropos, trajectory compression, skill system, memory providers) into agent-diva. Covers compatibility analysis and recommended architecture.
- **Status:** Planning — draft (v0.1.0-draft, 2026-04-05). Not yet implemented.

### 3. `hermes-learning/01-hermes-capabilities.md`
- **Title:** Hermes 自我学习能力详解
- **Summary:** Detailed breakdown of Hermes' self-learning capabilities: RL training pipeline (GRPO algorithm), trajectory data generation, skill system (procedural memory), and declarative memory. Includes Rust implementation considerations.
- **Status:** Research/analysis — completed reference document.

### 4. `hermes-learning/03-implementation-plan.md`
- **Title:** 融合架构设计与实施计划
- **Summary:** 13-18 week implementation roadmap for the Hermes learning fusion. Phase 1 covers core types and Hermes SessionDB. Phased delivery plan with weekly milestones.
- **Status:** Planning — draft. Not started.

### 5. `hermes-learning/README.md`
- **Title:** Hermes 自我学习机制集成 - README
- **Summary:** Index/README for the hermes-learning directory. Lists all documents in the series with brief descriptions. Status marked as "规划阶段" (planning stage).
- **Status:** Index document.

### 6. `genericagent-analysis.md`
- **Title:** GenericAgent 深度架构分析
- **Summary:** Deep architecture analysis of GenericAgent (~3K LOC Python), a self-evolving autonomous agent framework. Covers agent loop (Python generator + queue bus), tool chain, A2A capabilities, memory/learning, and comparison with agent-diva.
- **Status:** Research/analysis — completed (2026-06-01).

### 7. `hermes-agent-analysis.md`
- **Title:** Hermes Agent 运行时能力深度分析
- **Summary:** Runtime architecture analysis of Hermes as an agent framework (distinct from the learning integration docs). Covers "fat agent + module extraction" architecture, tool chain, A2A, skills system, memory persistence, and multi-platform channel support.
- **Status:** Research/analysis — completed (2026-06-01).

### 8. `claude-code-analysis.md`
- **Title:** Claude Code Agent 运行时架构深度分析
- **Summary:** Deep dive into Claude Code's internal architecture (v2.6.6 source). Core conclusion: Claude Code = one while-true loop + a highly refined harness layer. Covers tools, context compaction, subagent spawning, task system, worktree isolation, hooks, memory, and MCP routing.
- **Status:** Research/analysis — completed (2026-06-01).

### 9. `openharness-analysis.md`
- **Title:** OpenHarness 深度调研分析
- **Summary:** Analysis of OpenHarness, an open-source Python port of Claude Code by HKU. Covers 44 built-in tools, 10 chat platform integrations, multi-agent coordination (Swarm), evaluation architecture (3-layer evaluation system), and Docker sandbox execution.
- **Status:** Research/analysis — completed (2026-06-01).

### 10. `other-projects-analysis.md`
- **Title:** 其他项目深度分析: openfang / memtle / agent-diva-nano
- **Summary:** Comparative analysis of three related projects: openfang (Rust Agent OS, ~137K LOC), memtle (local-first memory palace), and agent-diva-nano (lightweight agent starter). Extracts design highlights for agent-diva evolution.
- **Status:** Research/analysis — completed (2026-06-01).

### 11. `codex-analysis.md`
- **Title:** OpenAI Codex CLI 架构深度分析
- **Summary:** Architecture analysis of OpenAI Codex CLI (Rust implementation). Covers agent loop (Op/Event-driven), tool chain, A2A capabilities, configuration, and comparison with Claude Code. Focused on providing orchestration reference for agent-diva.
- **Status:** Research/analysis — completed (2026-06-01).

### 12. `comparison-matrix.md`
- **Title:** 功能对比矩阵：7 大 Agent 项目全景对比
- **Summary:** Comprehensive feature comparison matrix across 7 agent projects (GenericAgent, Hermes, OpenHarness, Claude Code, Codex CLI, openfang, agent-diva). Compares agent loop, tool chain, A2A, memory, security, and more dimensions.
- **Status:** Research/synthesis — completed (2026-06-01).

### 13. `diva-capability-checklist.md`
- **Title:** agent-diva 功能清单：能力状态与差距分析
- **Summary:** Capability checklist derived from the 7-project comparison matrix. Categorizes agent-diva features as: already good, exists but insufficient, completely missing, or not needed. Assigns P0-P3 priorities.
- **Status:** Research/synthesis — completed (2026-06-01).

### 14. `evolution-roadmap.md`
- **Title:** agent-diva 演进路线建议
- **Summary:** Phased evolution roadmap for agent-diva based on the 7-project analysis. Strategic positioning as "Agent brain/orchestration layer." Phase 0 covers zero-cost improvements (tool timeouts, error classification, context budget). Later phases address deeper architecture changes.
- **Status:** Planning/recommendation — completed (2026-06-01).

### 15. `unknown-deficits.md`
- **Title:** agent-diva 未知缺陷分析
- **Summary:** Identifies hidden defects in agent-diva exposed by cross-project comparison. Highlights: infinite loop without circuit breaker, context overflow silent truncation, missing error classification, no iteration budget. Marked as "the most important output" of the research.
- **Status:** Research/analysis — completed (2026-06-01). High-priority actionable findings.

### 16. `sandbox-verification.md`
- **Title:** agent-diva-sandbox 安全能力验证报告
- **Summary:** Security verification of agent-diva-sandbox (v0.4.9, 13 crates). Checks shell command injection protection (regex deny list, 8 patterns) and streaming response interruption recovery. Finds partial implementation with gaps.
- **Status:** Audit — completed (2026-06-02).

### 17. `sandbox-audit-checklist.md`
- **Title:** 安全/沙箱/隔离/权限控制能力清单
- **Summary:** Comprehensive checklist of security capabilities extracted from all 6 analysis reports. Covers Docker sandbox, path/command permissions, shell escape prevention, tool timeouts, hook blocking, platform-level sandbox, approval strategies, sub-agent limits, etc.
- **Status:** Audit reference — completed (2026-06-02).

### 18. `sandbox-files-map.md`
- **Title:** Agent-Diva-Sandbox 安全/沙箱相关文件清单
- **Summary:** File-by-file inventory of security/sandbox-related files in agent-diva-sandbox. Maps core sandbox crate (lib.rs, policy.rs, exec_policy.rs, approval.rs, guardian.rs, orchestrator.rs, platform/*), tool layer security, and core security modules.
- **Status:** Audit reference — completed (2026-06-02).

### 19. `sandbox-audit-a.md`
- **Title:** 沙箱安全审计报告 — 维度 A：沙箱隔离 + Shell 安全
- **Summary:** Detailed audit of sandbox isolation and shell security. 9 items checked: 5 pass, 3 warning, 1 fail. Key finding: Windows RestrictedToken is dead code (not enabled), network egress control partial, subprocess env filtering missing.
- **Status:** Audit — completed (2026-06-02). Actionable findings.

### 20. `sandbox-audit-b.md`
- **Title:** Sandbox 审计报告 B：凭证安全 + 注入防护 + 熔断
- **Summary:** Audit of credential security, injection prevention, and circuit breakers. Finds: CLI config show has redaction but logging layer has no global credential scrubbing. Prompt injection scanning and MCP output filtering are partial/missing.
- **Status:** Audit — completed (2026-06-02). Actionable findings.

### 21. `sandbox-audit-c.md`
- **Title:** Sandbox 审计报告 C：子Agent安全 + MCP安全 + 审计
- **Summary:** Audit of sub-agent security and MCP safety. 11 items: 3 pass, 5 warning, 3 fail. Key failures: no global sub-agent concurrency control, MCP env var passthrough without filtering, no MCP request size limits.
- **Status:** Audit — completed (2026-06-02). Actionable findings.

### 22. `pro-ui-audit.md`
- **Title:** Agent Diva Pro — UI/UX 审计报告
- **Summary:** UI/UX audit of agent-diva-pro's Tauri + Vue.js GUI. Finds: dialog system exists but not used for tool approval; permission mode selector exists in frontend but not wired to backend; no Yes/No/Session three-tier approval; no sticky authorization cache.
- **Status:** Audit — completed (2026-06-02). Actionable findings.

### 23. `decisions.md`
- **Title:** agent-diva 能力增强决策记录
- **Summary:** Final decision record after user reviewed all 27 unknown defects and 24 capability gaps from the 7-project comparison. Prioritized into P0 (immediate), P1 (next sprint), P2 (planned). Includes sub-agent security suite, circuit breaker, path traversal fix, and more.
- **Status:** Decision record — completed (2026-06-01). Ready for implementation.

---

## Key research themes

### Theme 1: Cross-project Agent Architecture Analysis (7 projects)
**Files:** `genericagent-analysis.md`, `hermes-agent-analysis.md`, `claude-code-analysis.md`, `openharness-analysis.md`, `codex-analysis.md`, `other-projects-analysis.md`, `comparison-matrix.md`

A comprehensive survey of 7 agent frameworks (GenericAgent, Hermes, OpenHarness, Claude Code, Codex CLI, openfang, memtle/agent-diva-nano) to understand agent loop patterns, tool chains, A2A capabilities, memory systems, and security models. This is the foundational research that informed all subsequent analysis.

### Theme 2: Capability Gap Analysis & Evolution Planning
**Files:** `diva-capability-checklist.md`, `evolution-roadmap.md`, `unknown-deficits.md`, `decisions.md`

Synthesizes the cross-project research into actionable items for agent-diva. Identifies 27 unknown defects and 24 capability gaps. The `decisions.md` represents the final user-approved action plan with priorities (P0-P2). The `evolution-roadmap.md` provides a phased strategy.

### Theme 3: Hermes Self-Learning Integration
**Files:** `hermes-integration/00-current-architecture-analysis.md`, `hermes-learning/00-executive-summary.md`, `hermes-learning/01-hermes-capabilities.md`, `hermes-learning/03-implementation-plan.md`, `hermes-learning/README.md`

A focused research stream on integrating Hermes' self-learning (RL training, trajectory compression, skill system, memory providers) into agent-diva. Includes a 13-18 week implementation roadmap. All documents are marked as draft/planning stage — **none have been started**.

### Theme 4: Sandbox/Security Audit
**Files:** `sandbox-verification.md`, `sandbox-audit-checklist.md`, `sandbox-files-map.md`, `sandbox-audit-a.md`, `sandbox-audit-b.md`, `sandbox-audit-c.md`

A thorough security audit of agent-diva-sandbox across 3 dimensions (A: isolation + shell, B: credentials + injection + circuit breaker, C: sub-agent + MCP + audit). Results: multiple warnings and failures identified with specific file references.

### Theme 5: UI/UX Audit
**Files:** `pro-ui-audit.md`

Focused audit of agent-diva-pro's GUI (Tauri + Vue.js). Key finding: the permission mode UI exists but is not wired to the backend — a significant gap between frontend and backend security enforcement.

---

## Recommendations for TODOLIST

### Known gaps revealed by research
1. **Infinite loop without circuit breaker** (`unknown-deficits.md` Defect 1) — Agent can loop forever on repeated tool failures. No tool-call hash dedup or iteration budget. *Priority: P0 per decisions.md*
2. **Context overflow silent truncation** (`unknown-deficits.md` Defect 2) — No graceful degradation when context exceeds limits. *Priority: P0 per decisions.md*
3. **No sub-agent security suite** — Missing tool blacklist for delegated tasks, max depth limit, and credential minimization. *Priority: P0 per decisions.md*
4. **Sandbox Windows RestrictedToken is dead code** (`sandbox-audit-a.md`) — Implementation exists but is never called.
5. **Credential leakage in logs** (`sandbox-audit-b.md`) — No tracing Layer for global credential scrubbing. API keys can appear in logs.
6. **Permission mode UI not wired to backend** (`pro-ui-audit.md`) — Frontend permission selector is cosmetic only.
7. **No sub-agent concurrency control** (`sandbox-audit-c.md`) — Only QQ channel has max_concurrency; no global limit.
8. **MCP env var passthrough without filtering** (`sandbox-audit-c.md`) — Config.env is passed through unfiltered.

### Follow-up work needed
1. **Hermes learning integration** — 6 planning docs exist (all draft, 2026-04-05) but zero implementation has started. The 13-18 week roadmap needs a go/no-go decision.
2. **Sandbox audit remediation** — 3 audit reports (A/B/C) contain specific findings with file references. Each finding needs a fix-or-dismiss decision.
3. **Sub-agent timeout retry logic** — Marked P0-4 in decisions.md; needs implementation.
4. **Path traversal hardening** — P0-5a in decisions.md; add `..` traversal rejection before file operations.
5. **Tool execution timeout wrapping** — Phase 0 item in evolution-roadmap.md; wrap all tool executions with `tokio::time::timeout`.
6. **Error classification system** — Phase 0 item; replace simple try/catch with structured error categories (8+ levels like Hermes).

### Unfinished analysis
1. **Hermes integration feasibility** — The Hermes learning fusion plan (`hermes-learning/03-implementation-plan.md`) has a 13-18 week roadmap but no proof-of-concept or spike has been done.
2. **Sandbox audit completeness** — The audit covers 3 dimensions but explicitly notes gaps (e.g., subprocess env filtering is "❌" with no remediation plan yet).
3. **UI approval flow** — `pro-ui-audit.md` identifies the gap but no design doc exists for the approval UI → backend wiring.

### Decisions needing action
1. **Go/no-go on Hermes learning integration** — This is the largest pending research stream (6 docs, 13-18 week plan). Needs explicit decision: pursue, defer, or cancel.
2. **Sandbox audit P0 fixes** — `decisions.md` P0-1 (sub-agent security suite) and P0-2 (circuit breaker) need implementation commitment.
3. **Sandbox crate scope** — The sandbox audit reveals features that span `agent-diva-sandbox`, `agent-diva-core/src/security/`, and `agent-diva-tools/src/`. A clear ownership boundary needs to be established.
