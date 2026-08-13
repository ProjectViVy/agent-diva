# P0 Security/Stability Audit — agent-diva

**Date:** 2026-06-03
**Auditor:** Claude Code automated audit
**Scope:** Implementation status of 6 P0 items against design spec

---

### Item 1: Infinite Loop / Circuit Breaker
Status: **PARTIAL** (iteration cap only; no circuit breaker, no dedup, no rate limit)

Evidence:
  - `agent-diva-agent/src/agent_loop.rs:100` — `max_iterations: usize` field on `AgentLoop`
  - `agent-diva-agent/src/agent_loop/loop_turn.rs:120` — `'agent_loop: while iteration < self.max_iterations {` (hard loop guard, default 20)
  - `agent-diva-agent/src/subagent.rs:252` — sub-agents use hardcoded `max_iterations = 15`
  - `agent-diva-core/src/config/schema.rs:89` — `max_tool_iterations: u32` config field, default 20
  - `agent-diva-core/src/config/validate.rs:18-19` — validates `max_tool_iterations > 0`
  - `agent-diva-core/src/bus/events.rs:12` — `IterationStarted` event carries `index` and `max_iterations`
  - `agent-diva-cli/src/main.rs:881-888` — TUI renders iteration progress

Gap:
  - **No circuit breaker** — no `circuit_breaker` module/struct/function anywhere in source
  - **No tool-call hash dedup** — no `tool_call_hash`, no hash set tracking `(tool_name, args)` pairs; same failing tool can be called 20 times in a row
  - **No loop detection** — no `loop_detect`, no consecutive-failure counter, no detection of degenerate patterns
  - **No rate limiting on tool calls** — `SecurityPolicy` has `ActionTracker` for file ops (`security/rate_limit.rs`), but the agent loop has zero rate limiting on tool invocations
  - **No time-based budget** — no wall-clock timeout on the agent loop; a slow-but-non-terminating loop runs until `max_iterations` (20 LLM round-trips)
  - **No cost/token budget** — no `iteration_budget`, no token cost tracking during the loop

---

### Item 2: Sub-Agent Security Suite
Status: **PARTIAL** (tool blacklist exists; depth limit and credential minimization missing)

Evidence:
  - `agent-diva-agent/src/tool_config/builtin.rs:68-79` — `for_subagent()` hard-disables `spawn`, `cron`, `attachment`
  - `agent-diva-agent/src/tool_assembly.rs:109-115` — `build_subagent_registry()` nullifies `subagent_spawner`, `cron_service`, `file_manager`
  - `agent-diva-agent/src/tool_assembly.rs:241` — test `test_tool_assembly_subagent_mode_disables_spawn_and_attachment` validates blacklist
  - `agent-diva-agent/src/subagent.rs:252` — `max_iterations = 15` per sub-agent (iteration cap, not depth cap)
  - `agent-diva-agent/src/subagent.rs:339-377` — sub-agent system prompt states "cannot spawn", "cannot send messages", enforced via tool availability
  - `agent-diva-core/src/security/policy.rs` — `SecurityPolicy` with 8-layer path validation (used by file tools, NOT by sub-agent scope)
  - `agent-diva-core/src/security/rate_limit.rs` — `ActionTracker` sliding-window rate limiter (file operations only)

Gap:
  - **No max-depth limit** — no `max_depth`, no `depth` counter passed through spawn chain; recursion is prevented only by disabling the `spawn` tool (single-level). If `spawn` were ever re-enabled for sub-agents, there is no depth guard.
  - **No concurrent sub-agent cap** — `SubagentManager` tracks running tasks in `HashMap<String, JoinHandle>` (`subagent.rs:38`) and exposes `get_running_count()` (`subagent.rs:407`), but nothing enforces a maximum. Unbounded concurrent spawns are possible.
  - **No credential minimization** — sub-agents inherit the full LLM provider (with API key, `subagent.rs:112`), full `network_config` (with web search API key, `subagent.rs:117`), and full `mcp_servers` map (`subagent.rs:120`). No programmatic credential stripping or scoping.
  - **No configurable tool blacklist** — `for_subagent()` is hardcoded; users cannot define custom `disallowed_tools` lists per delegation context.
  - **SecurityPolicy does not govern sub-agents** — `SecurityPolicy` is used only by filesystem tools for path validation; it does not restrict what sub-agents can access.

---

### Item 3: Credential Scrubbing in Logs
Status: **NO** (display-layer redaction only; no tracing Layer for log scrubbing)

Evidence:
  - `agent-diva-cli/src/cli_runtime.rs:405-433` — `redact_sensitive_value()` replaces `api_key`/`token`/`secret`/`password` values with `***REDACTED***` — **only for `config show` command output**
  - `agent-diva-cli/tests/config_commands.rs:66-86` — integration test `config_show_json_redacts_secrets` validates CLI display redaction
  - `agent-diva-core/src/logging.rs` (157 lines) — central tracing subscriber with `EnvFilter` + stdout `fmt::layer()` + file `fmt::layer()` — **no custom Layer, no redaction**
  - `agent-diva-core/Cargo.toml:29` — `tracing-subscriber` with `["json"]` feature; no `"regex"` feature enabled
  - `Cargo.toml:41` — workspace `tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "local-time"] }` — no `regex` feature
  - `TODOLIST.md:19` — explicitly lists this as an open P0 item

Gap:
  - **No custom `tracing::Layer`** — zero implementations of `impl Layer for` / `fn on_event` / `fn on_record` anywhere in the codebase
  - **No regex-based scrubbing** — no pattern matching for `sk-*`, `Bearer *`, `ghp_*`, `xoxb-*` in log output
  - **Credentials leak in multiple paths:**
    - `agent-diva-providers/src/litellm.rs:555-558` — `Authorization: Bearer {api_key}` set on HTTP requests; debug logging can expose it
    - `agent-diva-providers/src/litellm.rs:742-761` — error response logging may echo auth headers
    - `agent-diva-core/src/config/schema.rs:779,796,1026` — `api_key` stored as plain `String`; any `Debug`/`Display` format prints cleartext
    - `agent-diva-core/src/error_context.rs:8` — `ErrorContext` truncates to 500 chars but does NOT redact credentials
    - `agent-diva-agent/src/tool_config/network.rs:24` — `api_key: Option<String>` cloned into tool args
  - **No `secrets_filter`** — no filter module, no scrubbing layer, no sensitive-field wrapper type

---

### Item 4: Session Truth-Source Fix (Phase A-PRE)
Status: **NO** (all 3 documented P0 bugs remain unfixed)

Evidence:
  - `agent-diva-core/src/session/manager.rs:103-126` — `save()` uses `std::fs::write()` (full overwrite, NOT atomic)
  - `agent-diva-agent/src/agent_loop/loop_turn.rs:63` — session loaded at start of turn
  - `agent-diva-agent/src/agent_loop/loop_turn.rs:466-479` — `save_turn()` adds messages to **in-memory** session only
  - `agent-diva-agent/src/agent_loop/loop_turn.rs:501-506` — `sessions.save()` persists to disk **after** entire turn completes
  - `agent-diva-agent/src/consolidation.rs:140-194` — consolidation writes MEMORY.md/HISTORY.md **before** `sessions.save()` runs
  - `agent-diva-core/src/memory/manager.rs:76` — `append_history()` uses full read-then-write (not atomic)
  - `agent-diva-gui/src\App.vue:471-504` — GUI localStorage cache with 30-min TTL (correctly treats backend as source of truth)
  - `agent-diva-files/src\backend.rs:167-172` — file service uses atomic temp+rename (pattern exists but NOT applied to sessions)
  - `docs\dev\agent-plan\phase-a-pre-session-truth-source-fix.md` — full plan document exists but is unimplemented
  - `docs\logs\2026-06-session-research\subagent-backend-audit.md` — documents 3 P0 bugs, all still present

Gap (3 documented P0 bugs):
  1. **User message lost on crash** — user message not persisted to disk until turn completes (line 501). Crash/kill between line 63 and 501 = message lost.
  2. **Split-brain on crash** — consolidation writes MEMORY.md/HISTORY.md (lines 482-498) BEFORE session JSONL save (line 501). Crash between = memory files advance ahead of session.
  3. **Non-atomic JSONL overwrite** — `std::fs::write()` truncates then writes; crash during write = total file corruption. No temp+rename, no backup, no fsync, no WAL.
  - **No crash recovery** — `load()` returns `None` on read failure, creating a new empty session that overwrites corrupt data
  - **No shutdown hook** — no flush of in-memory sessions on graceful exit
  - **No backup mechanism** — no `.bak` copy before overwrite

---

### Item 5: Path Traversal Hardening
Status: **YES** (comprehensive 8-layer implementation with tests)

Evidence:
  - `agent-diva-core/src/security/path.rs:10-149` — `PathValidator` with 8 validation layers:
    - Layer 1: `contains_null_bytes()` (line 10-11)
    - Layer 2: `contains_path_traversal()` using `Path::components()` + `Component::ParentDir` (line 14-19)
    - Layer 3: `contains_url_encoded_traversal()` — `..%2f`, `%2f..`, `..%5c`, `%5c..` (line 22-28)
    - Layer 4: `starts_with_tilde()` (line 31-33)
    - Layer 5: `is_absolute()` (line 36-38)
    - Layer 6: `matches_forbidden_prefix()` against configurable list (line 41-52)
    - `is_within_allowed_roots()` with canonicalization (line 63-84)
    - `validate_no_symlink_escape()` walks parent chain (line 87-123)
    - `is_extension_forbidden()` (line 133-139)
    - `sanitize_component()` strips `/`, `\`, null, `..` (line 142-149)
  - `agent-diva-core/src/security/policy.rs:84-137` — `is_path_allowed()` runs layers 1-7 sequentially
  - `agent-diva-core/src/security/policy.rs:173-233` — `validate_path()` and `validate_parent_directory()` with canonicalization + TOCTOU-safe pattern
  - `agent-diva-core/src/security/config.rs:54-100` — `SecurityConfig` with `workspace_only`, `forbidden_paths`, `forbidden_extensions`, `allow_symlinks`
  - `agent-diva-tools/src/filesystem.rs:90,215,340,447` — all 4 file tools call `security.validate_path()`
  - `agent-diva-tools/src/filesystem.rs:593-609` — test `test_path_traversal_blocked()` validates `../` rejection
  - `agent-diva-tools/src/shell.rs:123-148` — `SafetyGuard::validate()` rejects `..` in commands, canonicalizes extracted paths
  - `agent-diva-manager/src/skill_service.rs:322-343` — `normalize_archive_path()` rejects `Component::ParentDir` in zip entries
  - `agent-diva-manager/src/skill_service.rs:455-469` — test `upload_skill_zip_rejects_path_traversal()`
  - `agent-diva-gui/src-tauri/src/commands.rs:2375-2420` — GUI wipe validates against home/system dirs

Gap:
  - **TODOLIST.md line 28 is stale** — still marks path traversal as incomplete `[ ]` despite full implementation
  - Minor: MCP tool paths may not go through `SecurityPolicy` (MCP tools are external, so this may be by design)

---

### Item 6: Context Overflow Silent Truncation
Status: **NO** (no token budget, no context window awareness, no overflow detection)

Evidence:
  - `agent-diva-core/src/config/schema.rs:85` — `max_tokens: u32` default 8192 (output token limit only, NOT input/prompt budget)
  - `agent-diva-providers/src/litellm.rs:78-86` — `Usage` struct parses `prompt_tokens`, `completion_tokens`, `total_tokens` from responses — **never checked against any limit**
  - `agent-diva-tools/src/sanitize.rs:12-16` — `MAX_TOOL_RESULT_CHARS = 80,000` character-based truncation (blunt safety net, not budget system)
  - `agent-diva-tools/src/sanitize.rs:93-106` — `truncate_tool_result()` caps tool output at 80K chars
  - `agent-diva-agent/src/context.rs:308-319` — calls `truncate_tool_result()` when building messages
  - `agent-diva-agent/src/consolidation.rs:1-79` — consolidation triggered by message count (`unconsolidated >= memory_window`, default 100 messages), NOT token count
  - `agent-diva-core/src/error_context.rs:8-10` — `MAX_CONTEXT_LENGTH = 500` for error logging only

Gap:
  - **No `context_budget` / `token_budget`** — no config field, no runtime tracking
  - **No `context_window` configuration** — no way to declare model-specific context limits (e.g., 128K for GPT-4, 200K for Claude)
  - **No local tokenizer** — no tiktoken, no character-to-token estimation function
  - **No token counting** — prompt content is never measured before sending to API
  - **No sliding window** — no budget-aware message pruning or oldest-message dropping
  - **No context compression/compaction** — no summarize-on-overflow mechanism
  - **No overflow detection** — if conversation exceeds model context window, the provider returns a 400 error (hard failure, not graceful degradation)
  - **Usage data wasted** — `prompt_tokens` from API responses are parsed but never used for budget decisions

---

## Summary Table

| # | Item | Status | Risk |
|---|------|--------|------|
| 1 | Circuit Breaker / Loop Protection | PARTIAL | HIGH — only `max_iterations=20`, no dedup/detection |
| 2 | Sub-Agent Security Suite | PARTIAL | MEDIUM — tool blacklist works, no depth/credential controls |
| 3 | Credential Scrubbing in Logs | NO | HIGH — API keys can appear verbatim in log files |
| 4 | Session Truth-Source Fix | NO | HIGH — 3 documented P0 data-loss bugs, all unfixed |
| 5 | Path Traversal Hardening | YES | LOW — comprehensive 8-layer implementation with tests |
| 6 | Context Overflow Handling | NO | HIGH — no token budget, hard failure on overflow |

**Items requiring immediate attention:** 1, 3, 4, 6
**Item requiring follow-up:** 2
**Item complete:** 5 (TODOLIST.md should be updated to reflect this)
