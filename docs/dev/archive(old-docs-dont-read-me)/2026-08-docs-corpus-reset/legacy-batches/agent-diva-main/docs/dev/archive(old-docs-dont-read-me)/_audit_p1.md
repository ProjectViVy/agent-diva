# P1 Infrastructure Items — Implementation Audit

**Audit Date:** 2026-06-03
**Codebase:** agent-diva v0.5.0 (13 workspace crates)
**Auditor:** automated codebase scan

---

## Summary

| # | Item | Status |
|---|------|--------|
| 1 | Plan+TodoList implementation | **NO** |
| 2 | Phase B: Thin Observability Layer | **PARTIAL** |
| 3 | Sandbox audit remediation | **PARTIAL** |
| 4 | Permission mode UI wired to backend | **NO** |
| 5 | Tool execution timeout wrapping | **PARTIAL** |
| 6 | Error classification system | **PARTIAL** |
| 7 | SQLite storage for plan data | **NO** |
| 8 | 5-layer bypass prevention | **NO** |
| 9 | NAG mechanism | **NO** |

**Overall: 0/9 fully implemented, 4 partial, 5 not started.**

---

### Item 1: Plan+TodoList Implementation

**Status: NO**

**Evidence:**
- No `agent-diva-core/src/planning/` directory exists — `Glob("agent-diva-core/src/planning/**/*.rs")` returns zero files.
- No `agent-diva-agent/src/planning/` directory exists.
- No `agent-diva-tools/src/planning.rs` file exists.
- No types named `PlanOrchestrator`, `PlanStateStore`, `PlanVerifier`, `PlanPhase`, `PlanStatus`, `TodoList`, `TodoItem` appear anywhere in `.rs` files — `Grep("PlanOrchestrator|PlanStateStore|PlanVerifier|plan_start|plan_approve")` returns zero matches.
- Design docs are thorough and complete:
  - `docs/dev/agent-plan/plan-mode-architecture.md` — full MVP flow, data model, gate rules, component boundaries
  - `docs/dev/agent-plan/plan-todo-implementation-roadmap.md` — Option B (SQLite + Markdown projections) recommended, 4 implementation phases defined, SQL schema drafted
  - `docs/dev/agent-plan/plan-todo-ui-scope-extract.md` — UI scope extract exists

**Gap from spec:**
- Zero code implementation. All Phase 0–4 deliverables from the roadmap are unstarted:
  - No domain types (`Plan`, `PlanStep`, `TodoList`, `TodoItem`, `PlanEvent`)
  - No SQLite schema or migrations for `plans`, `plan_steps`, `todo_items`, `planning_events`, `active_plan`
  - No `PlanOrchestrator` lifecycle state machine
  - No CLI commands (`plan start`, `plan approve`, `plan status`, `plan verify`, `plan cancel`)
  - No manager HTTP endpoints (`/planning/status`, etc.)
  - No context injection of active plan/todolist into agent prompts
  - No planning tools (`plan_show`, `todo_show`, `todo_update`, etc.)

---

### Item 2: Phase B: Thin Observability Layer

**Status: PARTIAL**

**Evidence of existing infrastructure:**
- `agent-diva-core/src/logging.rs` — Full `tracing-subscriber` setup with:
  - `EnvFilter` with per-module overrides (`logging.rs:24-35`)
  - JSON and text format modes (`logging.rs:57-98`)
  - Daily rolling file appender (`logging.rs:47`)
  - Thread IDs, file names, line numbers in output (`logging.rs:63-64, 85-86`)
  - 7-day log cleanup (`logging.rs:109, 117-156`)
- `Cargo.toml:40-42` — `tracing`, `tracing-subscriber` (env-filter, fmt, local-time), `tracing-appender` as workspace deps
- `agent-diva-agent/src/agent_loop.rs:468-470` — `trace_id` generated as `Uuid::new_v4()` and attached to `tracing::info_span!("AgentSpan", trace_id = %trace_id)`
- `agent-diva-agent/src/agent_loop/loop_turn.rs` — 12 structured trace points with `trace_id`, `step_name`, `loop_index`, `tool_name` fields (lines 35, 109, 112, 129, 300, 323, 393, 447, 515, 517)
- `agent-diva-neuron/src/events.rs:10` — `trace_id` field in event struct
- `agent-diva-neuron/src/executor.rs:59` — `trace_id` generated for executor events

**Evidence of gaps (per spec in `docs/dev/Observability/phase-b-thin-observability-layer.md`):**
- No `agent-diva-core/src/trace/` module — `Glob("agent-diva-core/src/trace/**/*.rs")` returns zero files
- No typed `TraceId` struct — trace_id is a raw `String` (`Uuid::new_v4().to_string()`)
- No typed `TraceEvent` struct with required fields (`ts`, `level`, `trace_id`, `session_id`, `channel`, `component`, `event`, `summary`, `metadata`)
- No append-only JSONL writer for structured runtime events (`.agent-diva/logs/runtime-YYYY-MM-DD.jsonl`)
- No redaction layer — `agent-diva-core/src/logging.rs` has no secret filtering; audit report `sandbox-audit-b.md` confirms "日志层全局脱敏 ❌ — 无 tracing Layer 过滤"
- No `tracing::Layer` implementation to intercept and redact `sk-*`, `Bearer *`, API keys from log output
- No retention policy beyond the 7-day file cleanup (no structured log retention)
- No debug bundle export mechanism
- No gateway-specific structured events (`gateway_inbound`, `gateway_outbound`, `gateway_error`)
- No minimal event set emission (`message_received`, `llm_request_started`, `tool_call_started`, etc. as structured events)
- The existing `trace!()` calls use ad-hoc fields, not the spec's required top-level schema

**Gap from spec:**
- Phase B0 (trace types + JSONL writer + redaction): **not started**
- Phase B1 (agent runtime structured events): **partially via ad-hoc tracing spans, not spec-compliant**
- Phase B2 (gateway events): **not started**
- Phase B3 (debug bundle + settings): **not started**

---

### Item 3: Sandbox Audit Remediation

**Status: PARTIAL**

**Context:** Three audit reports exist (`sandbox-audit-a.md`, `sandbox-audit-b.md`, `sandbox-audit-c.md`) referencing `agent-diva-sandbox/` as a crate. However, **no `agent-diva-sandbox` crate exists in the workspace** — `Cargo.toml` members list has no sandbox entry, and `Glob("agent-diva-sandbox/**/*")` returns zero files. The audit reports describe a planned or external sandbox crate that is not part of this codebase.

**What IS implemented in-workspace (core security):**

| Component | Status | File |
|-----------|--------|------|
| `SecurityPolicy` with 8-layer path validation | ✅ | `agent-diva-core/src/security/policy.rs` |
| `PathValidator` (null bytes, traversal, URL-encoded, tilde, absolute, forbidden prefix, extension, canonicalize) | ✅ | `agent-diva-core/src/security/path.rs` |
| `SecurityConfig` with levels (Permissive/Standard/Strict/Paranoid) | ✅ | `agent-diva-core/src/security/config.rs` |
| `ActionTracker` sliding-window rate limiting | ✅ | `agent-diva-core/src/security/rate_limit.rs` |
| `SecurityError` structured error types | ✅ | `agent-diva-core/src/security/error.rs` |
| Shell deny patterns (rm -rf, format, fork bomb, etc.) | ✅ | `agent-diva-tools/src/shell.rs:84-99` |
| Shell workspace restriction (path traversal, absolute paths) | ✅ | `agent-diva-tools/src/shell.rs:102-151` |
| File size limits (10MB default) | ✅ | `agent-diva-core/src/security/config.rs:102` |
| Forbidden paths (`/etc`, `~/.ssh`, `~/.aws`, etc.) | ✅ | `agent-diva-core/src/security/config.rs:84-92` |
| Forbidden extensions (`.exe`, `.dll`, `.bat`, `.cmd`, `.sh`) | ✅ | `agent-diva-core/src/security/config.rs:94-100` |
| Symlink restrictions | ✅ | `agent-diva-core/src/security/policy.rs:192-198` |

**What is NOT implemented (from audit findings):**

| Finding | Status | Source |
|---------|--------|--------|
| Platform-level sandbox (Windows RestrictedToken, Linux Landlock/bwrap/Seccomp, macOS Seatbelt) | ❌ | sandbox-audit-a §1 — no sandbox crate in workspace |
| Environment variable filtering for subprocesses | ❌ | sandbox-audit-a §9 — `shell.rs` does not call `env_clear()` |
| Prompt injection scanning | ❌ | sandbox-audit-b §2 — "完全缺失" |
| Memory threat pattern scanning | ❌ | sandbox-audit-b §3 — memory writes are unfiltered |
| Tool execution failure circuit breaker | ❌ | sandbox-audit-b §4 — only approval-rejection breaker exists (in sandbox crate) |
| Credential redaction in logs | ❌ | sandbox-audit-b §1 — no tracing Layer for redaction |
| Subagent tool blacklist | ❌ | sandbox-audit-c §1 — subagents inherit full parent tool set |
| Subagent concurrency limit | ❌ | sandbox-audit-c §3 — no `Arc<Semaphore>` or similar |
| Subagent turn-level timeout | ❌ | sandbox-audit-c §4 — only MCP timeout exists |
| MCP env variable filtering | ❌ | sandbox-audit-c §6 — `config.env` passed through |
| MCP request size limit | ❌ | sandbox-audit-c §7 |
| Structured security audit events | ❌ | sandbox-audit-c §10 |

**Gap from spec:**
- The `agent-diva-sandbox` crate referenced in audits does not exist in the workspace. Core security primitives (path validation, rate limiting, config) are solid, but all platform-level isolation, approval gates, guardian/circuit-breaker, and advanced security features described in the audits are absent from the built codebase.

---

### Item 4: Permission Mode UI Wired to Backend

**Status: NO**

**Evidence:**
- `Grep("permission|approval|authorization|sticky")` in `agent-diva-gui/` — zero matches in `.rs` files
- `Grep("permission|approval|authorization|sticky")` in `agent-diva-manager/` — zero matches in `.rs` files
- `Grep("yes.*no.*session|three.*tier|sticky.*auth|approval.*mode")` across all `.rs` files — zero matches
- No three-tier approval model (Yes/No/Session) exists anywhere in the codebase
- No sticky authorization persistence mechanism
- The sandbox audit checklist (`sandbox-audit-checklist.md:21-26`) describes Codex CLI's approval model as a reference: `Never/OnRequest/UnlessTrusted/OnFailure/Granular` modes with `ExecApprovalRequest` and sticky persistence — none of this is implemented

**Gap from spec:**
- No approval mode enum or configuration
- No approval request/response flow between tools and UI
- No sticky authorization store (per-command, per-session caching of user decisions)
- No GUI components for approval prompts
- No manager API endpoints for approval state
- No integration with tool execution pipeline (pre-execution gate)

---

### Item 5: Tool Execution Timeout Wrapping

**Status: PARTIAL**

**Evidence of timeouts:**

| Component | Has Timeout | Mechanism | File:Line |
|-----------|------------|-----------|-----------|
| Shell tool (`ExecTool`) | ✅ | `tokio::time::timeout` built into `execute()` | `agent-diva-tools/src/shell.rs:11` (import), timeout_secs field at line 50 |
| MCP tool calls | ✅ | `tokio::time::timeout(timeout_duration, ...)` on `request_tool_call` | `agent-diva-tools/src/mcp_sdk.rs:246-255` |
| MCP tool list | ✅ | `tokio::time::timeout` on `request_tool_list` | `agent-diva-tools/src/mcp_sdk.rs:215-217` |
| MCP client startup | ✅ | `tokio::time::timeout` clamped 10–120s | `agent-diva-tools/src/mcp_sdk.rs:196-198` |
| Filesystem tools | ❌ | No timeout | `agent-diva-tools/src/filesystem.rs` — no `tokio::time::timeout` |
| Web tools | ❌ | No timeout wrapping | `agent-diva-tools/src/web.rs` — no `tokio::time::timeout` |
| Spawn tool | ❌ | No timeout | `agent-diva-tools/src/spawn.rs` |
| Message tool | ❌ | No timeout | `agent-diva-tools/src/message.rs` |
| **ToolRegistry::execute()** | ❌ | **No global timeout wrapper** | `agent-diva-tooling/src/registry.rs:52-121` — calls `tool.execute()` directly with no timeout |

**Critical gap:** The `ToolRegistry::execute()` method at `agent-diva-tooling/src/registry.rs:89` calls `tool.execute(params.clone()).await` with **no `tokio::time::timeout` wrapper**. The `exec_timeout` value is configured and passed through `ToolAssembly` → `ExecTool`, but only the shell tool uses it internally. There is no registry-level or agent-loop-level timeout that wraps all tool executions.

The agent loop at `agent-diva-agent/src/agent_loop/loop_turn.rs:367` calls `self.tools.execute(&tool_call.name, params_value).await` with no timeout — a hung tool (e.g., a filesystem operation on a network mount) would block the agent loop indefinitely.

**Gap from spec:**
- No global `tokio::time::timeout` in `ToolRegistry::execute()` wrapping all tool calls
- No per-tool configurable timeout at the registry level (only shell and MCP have their own)
- Filesystem, web, spawn, and message tools have no timeout protection
- No agent turn-level total timeout (as noted in sandbox-audit-c §4)

---

### Item 6: Error Classification System

**Status: PARTIAL**

**Evidence:**

**`ToolError` enum** (`agent-diva-tooling/src/base.rs:59-75`):
```rust
pub enum ToolError {
    Error(String),           // generic
    InvalidParams(String),   // parameter validation
    InvalidArguments(String),// argument validation
    ExecutionFailed(String), // execution failure
    Io(std::io::Error),      // IO error
}
```

**`SecurityError` enum** (`agent-diva-core/src/security/error.rs:8-48`):
- 9 structured variants: `PathNotAllowed`, `PathEscapesWorkspace`, `ForbiddenComponent`, `RateLimitExceeded`, `ActionBudgetExhausted`, `ReadOnlyMode`, `SymlinkNotAllowed`, `InvalidPathFormat`, `FileTooLarge`, `ForbiddenExtension`
- Has `user_message()` for user-friendly display
- Has `is_retryable()` classification

**`ErrorContext`** (`agent-diva-core/src/error_context.rs` referenced in `agent-diva-tooling/src/registry.rs:4`):
- Provides contextual error enrichment with metadata, content truncation
- Used in `ToolRegistry::execute()` for error wrapping

**`FileError`** (`agent-diva-files/src/lib.rs:97`):
- Has `From<sqlx::Error>` conversion

**Gaps:**
- No unified error taxonomy across crates — `ToolError` has only 5 flat variants with no error codes, categories, or severity
- No `error_category` or `ErrorKind` classification system (search returns zero matches for these patterns)
- No error classification for: network errors, timeout errors, authentication errors, rate limit errors, provider errors, tool-not-found errors (currently just string formatting)
- No structured error codes for programmatic handling
- `ToolError` variants are mostly string-based (`Error(String)`, `ExecutionFailed(String)`) — callers cannot match on specific failure modes
- No retry/no-retry classification at the `ToolError` level (only `SecurityError` has `is_retryable()`)

**Gap from spec:**
- Need structured error categories (at minimum: `Timeout`, `PermissionDenied`, `NotFound`, `Network`, `Provider`, `Validation`, `Internal`)
- Need error codes for programmatic handling by manager/CLI/GUI
- Need retry classification on `ToolError`
- Need unified error type or conversion between `ToolError`, `SecurityError`, and provider errors

---

### Item 7: SQLite Storage for Plan Data

**Status: NO**

**Evidence:**
- `Cargo.toml:86` — `sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "migrate", "chrono"] }` — SQLite support IS available as a workspace dependency
- `agent-diva-files/Cargo.toml:27` — `sqlx = { workspace = true }` — currently the ONLY crate using sqlx
- `agent-diva-files/src/index.rs` — SQLite used for file metadata index (`CREATE TABLE IF NOT EXISTS files`)
- `agent-diva-files/src/channel.rs` — SQLite used for channel file tracking (`CREATE TABLE IF NOT EXISTS channel_files`)
- **No planning database**: `Grep("sqlx|sqlite|CREATE TABLE")` in all crates — only `agent-diva-files` has SQLite tables
- No `planning.db` file creation, no `plans`/`plan_steps`/`todo_items`/`planning_events`/`active_plan` tables
- The recommended schema from `plan-todo-implementation-roadmap.md` (5 tables, lines 181-242) is fully designed but zero percent implemented

**Gap from spec:**
- Schema for `plans`, `plan_steps`, `todo_items`, `planning_events`, `active_plan` tables — not created
- No SQLite store initialization for planning data
- No migration files for planning tables
- No CRUD operations for plan/todo entities
- The infrastructure (sqlx + SQLite feature) is ready — only the planning-specific schema and code are missing

---

### Item 8: 5-Layer Bypass Prevention

**Status: NO**

**Evidence:**
- `Grep("bypass|intercept|audit_log|plan_state")` in all `.rs` files — only `agent-diva-files/src/manager.rs` matches (for file manager hooks, not plan bypass prevention)
- `Grep("PlanOrchestrator|PlanStateStore|PlanVerifier|plan_state")` — zero matches
- No `PlanOrchestrator` component exists (see Item 1)
- No audit log system for plan state transitions
- No filesystem intercept layer that validates plan state before tool execution
- No tool-layer gate that checks "is there an active plan? is this tool call allowed in current phase?"
- No plan state machine that enforces Explore → Plan → AwaitingApproval → Execute → Verify transitions

**The 5 layers from the spec and their status:**

| Layer | Description | Status |
|-------|-------------|--------|
| 1. Filesystem intercept | Block file mutations that bypass plan evidence writes | ❌ Not implemented |
| 2. Tool layer gate | ToolRegistry checks plan state before executing mutating tools | ❌ Not implemented |
| 3. PlanOrchestrator | Central state machine enforcing phase transitions and approval gates | ❌ Not implemented |
| 4. Audit log | Append-only `events.jsonl` recording all plan state transitions | ❌ Not implemented |
| 5. Context injection | Agent context includes plan phase + current todo, preventing model from bypassing | ❌ Not implemented |

**Gap from spec:**
- All 5 layers are unimplemented. This is expected since Plan Mode itself (Item 1) is not implemented — bypass prevention is a dependency of the Plan Mode architecture.

---

### Item 9: NAG Mechanism

**Status: NO**

**Evidence:**
- `Grep("nag|reminder|inject.*planning|planning_tool")` in all `.rs` files — zero matches for nag/reminder-injection patterns
- `Grep("nag|reminder|inject|planning_tool")` in `agent-diva-agent/src/` — matches are all unrelated:
  - `agent-diva-agent/src/context.rs:152` — cron tool reminder text ("When a user asks to create a reminder...")
  - `agent-diva-agent/src/agent_loop/loop_turn.rs:78` — cron trigger notification text
  - `agent-diva-agent/src/subagent.rs:325` — "Inject as system message to trigger main agent" (subagent completion injection)
- No counter tracking "rounds without planning tool usage"
- No system message injection when planning tools are available but unused
- No planning-aware heuristic in the agent loop

**Gap from spec:**
- No mechanism to detect that the agent has been executing for N rounds without invoking a planning tool
- No automatic reminder injection ("Consider using plan_show or todo_update to track your progress")
- No configurable N threshold (spec suggests 3 rounds)
- This depends on planning tools (Item 1) existing first — the NAG mechanism is meaningless without tools to nag about

---

## Cross-Cutting Dependencies

```
Item 7 (SQLite) ──→ Item 1 (Plan+TodoList) ──→ Item 8 (Bypass Prevention)
                                              ──→ Item 9 (NAG Mechanism)
                                              ──→ Item 4 (Permission UI)

Item 2 (Observability) ──→ independent, can proceed now
Item 5 (Timeout) ──→ independent, can proceed now
Item 6 (Error Classification) ──→ independent, can proceed now
Item 3 (Sandbox Remediation) ──→ partially independent (core security exists)
```

## Recommended Priority Order

1. **Item 1 + 7** (Plan+TodoList + SQLite) — foundational for items 4, 8, 9
2. **Item 5** (Tool timeout wrapping) — quick win, high safety impact, add `tokio::time::timeout` in `ToolRegistry::execute()`
3. **Item 2** (Observability) — Phase B0 (TraceId + JSONL writer + redaction) is high-value and independent
4. **Item 6** (Error classification) — enrich `ToolError` with structured variants
5. **Item 3** (Sandbox remediation) — implement env filtering, credential redaction, subagent limits
6. **Items 4, 8, 9** (Permission UI, Bypass Prevention, NAG) — depend on Item 1
