# Adversarial Review v2 — PRD: Agent-Diva Pro 报表系统 & Session 历史检索

> Reviewer: adversarial validator  
> Date: 2026-06-08  
> Scope: Re-validation of P0 fixes + outstanding P1/P2 issues  
> Files reviewed: `prd.md`, `.decision-log.md`

---

## Overall Verdict

**CONDITIONAL PASS with 7 outstanding risks.**

The P0 fixes (decision log closure, GUI commit hash + file size, session atomic write) are mechanically present but remain thin. The PRD is now internally consistent but still lacks engineering rigor in critical areas: LLM cost/retry, migration path from regex to semantic search, and acceptance criteria across all FRs. Several P2 items (circular assumptions, subjective quality gates, storage path collisions, lifecycle management, FR-10 cop-out) remain unaddressed and would block a production release.

---

## P0 Fix Validation

| ID | Fix Claim | Status | Assessment |
|----|-----------|--------|------------|
| P0-1 | Decision Log Open Items → Closed | ✅ Fixed | All 5 items now Closed with references to PRD §8 decisions. Log is consistent. |
| P0-2 | GUI "已就绪" → includes commit `fcf768d` and file size | ⚠️ Partially Fixed | PRD §6.1 states "GUI 已就绪，commit `fcf768d`，NotebookView.vue 725 行." The file size (725 lines) is present but this is a weak readiness signal. No evidence of component tests, integration tests, or visual regression tests is mentioned. "已就绪" remains unverifiable without test coverage. |
| P0-3 | Session atomic write → describes actual non-atomic write problem and fix | ✅ Fixed | PRD §6.1 now correctly identifies `SessionManager::save` as non-atomic (direct `.jsonl` overwrite) and specifies the fix: temp file + rename. References `agent-diva-core/src/session/manager.rs:124`. |

**P0 Verdict**: P0-1 and P0-3 are sufficiently resolved. P0-2 is technically present but the "已就绪" claim remains weakly substantiated.

---

## P1 Issues (Previously Flagged) — Current Status

### P1-1: LLM Cost / Retry Logic Still Missing

**Status**: ❌ **Still Missing**

- PRD §11.3 (Cost) mentions "控制调用频率" but provides no concrete limits.
- No retry policy (exponential backoff? max retries? fallback model?)
- No cost estimation or budget cap mechanism.
- No mention of token usage tracking or rate limiting.
- **Risk**: LLM API failures (rate limits, timeouts, model unavailability) will cascade into silent report generation failures. The "记录错误日志" in FR-1 is insufficient without retry/recovery.
- **Required**: Add explicit retry policy (e.g., 3 retries with exponential backoff, fallback to cheaper model), token budget per report, and circuit breaker pattern.

### P1-2: Migration Path from Regex to Semantic Search Still Missing

**Status**: ❌ **Still Missing**

- PRD §5 (Non-Goals) defers semantic search to v2.
- PRD §6.2 Out of Scope table lists "语义搜索" with reason "需要 embedding 模型" and plan "v2".
- **No migration path** is defined: no data model for embeddings, no indexing strategy, no backward compatibility plan for existing regex-based search results, no deprecation timeline for regex search.
- **Risk**: v2 semantic search will require a full re-index of historical sessions. Without a migration path, this becomes a breaking change or a massive one-time migration.
- **Required**: Define the data model (e.g., add `embedding` field to session metadata), indexing strategy (incremental vs. full re-index), and a phased rollout plan (dual-write period, feature flag).

### P1-3: Acceptance Criteria Still Missing from All FRs

**Status**: ❌ **Still Missing**

- None of the 10 FRs (FR-1 through FR-10) contain acceptance criteria.
- "Consequences" sections describes behavior but does not define "done".
- **Example gap**: FR-1 (自动日报生成) has no criteria for:
  - Minimum content coverage (e.g., must cover ≥80% of sessions)
  - Maximum latency (e.g., generation must complete within 30s for 100 sessions)
  - Quality threshold (e.g., hallucination rate < 5%)
  - Error handling (e.g., what happens when LLM returns malformed markdown?)
- **Required**: Add acceptance criteria to every FR using Given-When-Then or similar structured format. Minimum: success criteria, error criteria, performance criteria.

---

## P2 Issues (Previously Flagged) — Current Status

### P2-1: ASSUMPTION-4 Circular

**Status**: ❌ **Still Present**

> ASSUMPTION-4: "历史搜索的性能瓶颈在可接受范围内（短期方案）"

- This is a self-referential assumption: it assumes the problem it purports to address is not a problem.
- No definition of "可接受范围" (acceptable range). Is 5s for 100 sessions still acceptable at 1,000 sessions? 10,000?
- No benchmark or measurement plan.
- **Risk**: Performance degradation will be discovered only in production. The "短期方案" regex approach (FR-9) will O(n) degrade with session growth.
- **Required**: Replace with a measurable assumption (e.g., "搜索响应时间 p99 < 5s for up to 5,000 sessions, based on benchmark X") or downgrade to a known risk with a mitigation plan.

### P2-2: ASSUMPTION-3 Subjective

**Status**: ❌ **Still Present**

> ASSUMPTION-3: "用户对报表内容的质量要求以'可用'为标准，而非'完美'"

- "可用" is undefined and unverifiable. One user's "可用" is another user's "unusable".
- No quality metric (e.g., user satisfaction score, error rate, human review sample) is tied to this assumption.
- **Risk**: Disputes over quality will block release or trigger rework.
- **Required**: Define "可用" operationally (e.g., "≥80% of generated reports require no manual correction before user accepts them, measured via thumbs-up/down feedback").

### P2-3: Storage Path Collisions

**Status**: ❌ **Still Present**

- PRD specifies deterministic paths:
  - `{workspace}/reports/daily/{YYYY-MM-DD}.md`
  - `{workspace}/reports/weekly/{YYYY-Www}.md`
  - `{workspace}/reports/monthly/{YYYY-MM}.md`
  - `{workspace}/sops/{report-id}.md`
  - `{workspace}/skills/{skill-name}.md`
- **No collision handling**: What happens when a report is regenerated? When a skill name conflicts with an existing file? When multiple users share a workspace?
- FR-4 mentions "强制刷新" (force refresh) but does not define the overwrite behavior (versioning? backup? prompt?)
- **Risk**: Data loss from accidental overwrites, or silent failures when files are locked.
- **Required**: Define collision policy (e.g., versioned filenames, atomic swap with backup, user confirmation for overwrites).

### P2-4: Lifecycle Management

**Status**: ❌ **Still Present**

- No retention policy for reports, SOPs, or skills.
- No archival or cleanup mechanism.
- `{workspace}/reports/` will grow unbounded.
- **Risk**: Disk space exhaustion, degraded GUI performance (long file lists), and compliance issues (if reports contain PII).
- **Required**: Define retention rules (e.g., daily reports retained for 30 days, weekly for 90 days, monthly for 1 year) and automatic cleanup or archival.

### P2-5: FR-10 Visualization Cop-Out

**Status**: ❌ **Still Present**

> FR-10: "不要求可视化展示（可通过 API/命令行返回）"

- This is a user-facing feature (search results) with no GUI representation.
- UJ-2 (小红搜索数据库优化讨论) describes a natural language interaction, but FR-10 says results can be returned as "文本列表或结构化 JSON" with no visualization.
- **Risk**: Poor UX. Users will receive raw JSON or plain text in a chat interface, which is hard to scan and act upon.
- **Required**: Either commit to a minimal visualization (e.g., collapsible session cards in the GUI) or explicitly scope this as a CLI-only MVP feature with GUI deferred to v2.

---

## New Issues Discovered in v2 Review

### N1: Missing Error Handling for Report Generation

- FR-1 mentions "日报生成失败时，记录错误日志" but does not define:
  - What constitutes a failure (LLM timeout? malformed response? disk full?)
  - How the user is notified (GUI toast? silent log?)
  - Whether failed reports are retried or skipped
- **Risk**: Silent failures will erode user trust.

### N2: Missing Concurrency Control

- If multiple reports are triggered simultaneously (e.g., daily + weekly + monthly on the first of the month), there is no mention of:
  - Deduplication (will the same session be summarized 3 times?)
  - Resource throttling (LLM API rate limits)
  - Locking mechanism (preventing concurrent writes to the same report file)
- **Risk**: Race conditions, duplicate LLM calls, and inconsistent report state.

### N3: Inconsistent Time Zone Handling

- PRD specifies 00:00 trigger but does not specify the time zone (UTC? local system time? user preference?)
- "昨日聊天记录" depends on time zone boundary.
- **Risk**: Reports generated at incorrect boundaries for users in different time zones.

---

## Summary Table

| Category | Issue | Status | Severity |
|----------|-------|--------|----------|
| P0 | Decision Log closure | ✅ Fixed | — |
| P0 | GUI readiness (commit + size) | ⚠️ Partial | Low |
| P0 | Session atomic write | ✅ Fixed | — |
| P1 | LLM cost/retry | ❌ Missing | **High** |
| P1 | Regex → semantic migration | ❌ Missing | **High** |
| P1 | Acceptance criteria | ❌ Missing | **High** |
| P2 | ASSUMPTION-4 circular | ❌ Unchanged | Medium |
| P2 | ASSUMPTION-3 subjective | ❌ Unchanged | Medium |
| P2 | Storage path collisions | ❌ Unchanged | Medium |
| P2 | Lifecycle management | ❌ Unchanged | Medium |
| P2 | FR-10 visualization | ❌ Unchanged | Medium |
| New | Error handling for generation | ❌ Missing | Medium |
| New | Concurrency control | ❌ Missing | Medium |
| New | Time zone handling | ❌ Missing | Low |

---

## Recommendations

1. **Block release until P1 issues are addressed**: LLM retry logic, migration path, and acceptance criteria are non-negotiable for a production PRD.
2. **Add acceptance criteria to all FRs**: Use a consistent template (Given-When-Then + performance + error thresholds).
3. **Replace subjective assumptions with measurable ones**: Define "可用" and "可接受范围" with numbers.
4. **Define storage collision and lifecycle policies**: These are table-stakes for any file-based system.
5. **Scope FR-10 properly**: Either commit to GUI visualization or explicitly mark as CLI-only for MVP.
6. **Address N1-N3**: Error handling, concurrency, and time zones are typically discovered late but are cheap to specify early.

---

*End of Review*
