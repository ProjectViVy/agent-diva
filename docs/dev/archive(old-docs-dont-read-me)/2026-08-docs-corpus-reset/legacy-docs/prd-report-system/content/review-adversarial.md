# Adversarial Review — Agent-Diva Pro 报表系统 & Session 历史检索 PRD

> Reviewer: Adversarial Reviewer  
> Date: 2026-06-08  
> Scope: `prd.md` + `.decision-log.md`  
> Status: **BLOCKING ISSUES FOUND — Do not proceed to implementation without resolution**

---

## Executive Summary

This PRD describes a report generation and session history search feature for Agent-Diva Pro. While the user journeys are well-intentioned, the document contains **critical gaps in operational readiness**, **unverified assumptions**, **undefined acceptance criteria**, and **data inconsistencies** between the PRD and its decision log. Several items marked as MVP-ready are not actually ready.

**Verdict: REJECT for implementation until P0/P1 items below are resolved.**

---

## P0 — Blocking (Must Fix Before Implementation)

### P0-1. Decision Log Contradiction: "Confirmed" vs "Open"

**Finding**: All 5 Open Questions in PRD §8 are marked "已确认" (confirmed), yet the Decision Log lists the **exact same 5 items** as "Open" with "Revisit Condition" set to implementation time.

| PRD §8 | Decision Log |
|--------|------------|
| 日报生成时间 → 已确认 | 日报生成具体时间 → Open |
| 周报起始日 → 已确认 | 周报起始日 → Open |
| SOP/Skill 格式 → 已确认 | SOP/Skill 具体格式 → Open |
| Memory Provider → 已确认 | Memory Provider 选择 → Open |
| 搜索权限 → 已确认 | 搜索权限控制 → Open |

**Impact**: This is not a minor inconsistency — it means either:
- The PRD was updated without updating the decision log (process failure), OR
- The "confirmations" are premature and the items are genuinely still open (substance failure).

Either way, **downstream teams cannot trust the PRD's "confirmed" status**.

**Recommendation**: Reconcile both documents. If truly confirmed, move Decision Log items to "Closed" with reference to the PRD commit. If still open, remove "已确认" from PRD §8 and keep them as actual open questions.

---

### P0-2. "GUI 已就绪" Claim is Unverified

**Finding**: PRD §6.1 In Scope states: "报表查看（GUI 已就绪）". No evidence is provided in the PRD or references to support this claim.

**Questions unanswered**:
- What does "已就绪" mean? Code-complete? Merged to main? Tested? UI/UX-reviewed?
- Is `NotebookView.vue` (referenced in §12) actually functional, or is it a stub?
- Does the GUI handle empty states, loading states, and error states for report viewing?

**Impact**: If the GUI is not actually ready, this becomes a hidden dependency that blocks the entire MVP.

**Recommendation**: Add a reference to the GUI component's current status (commit hash, test coverage, design review sign-off). If it's a stub, move "GUI 报表查看" to Out of Scope or create a separate tracking issue.

---

### P0-3. Session Atomic Write — Technical Debt Insufficiently Assessed

**Finding**: PRD §6.1 includes "Session 原子写入修复（cherry-pick main 的 `write_session_atomically`）" as In Scope, but provides no detail on:
- What is broken in the current session write path?
- What are the risks of cherry-picking vs. proper merge/rebase?
- Who owns the `write_session_atomically` change on main?
- What is the rollback plan if the cherry-pick introduces regressions?
- How will this be tested before MVP release?

**Impact**: Cherry-picking core persistence logic is high-risk. If this fails, session data loss is a real possibility. The PRD treats this as a one-liner task.

**Recommendation**: Expand this to a mini-RFC or at minimum a risk assessment paragraph. Include: scope of change, test plan, rollback strategy, and owner.

---

## P1 — High Risk (Fix Before v1 Release)

### P1-1. LLM Token Cost & Failure Retry Strategy Completely Missing

**Finding**: The PRD acknowledges报表生成依赖 LLM (§9 ASSUMPTION-2, §11.3) but the NFR section (§10) contains **zero mention** of:
- Token cost budget or cost-per-report estimates
- LLM API failure handling (rate limits, timeouts, model unavailability)
- Retry strategy (exponential backoff? circuit breaker? max retries?)
- Fallback behavior (what happens if LLM is down for 6 hours?)
- Cost attribution (who pays? is there a spending cap?)

**Impact**: Without these, the feature is not production-ready. A single runaway cron job could exhaust API quota or rack up unexpected costs.

**Recommendation**: Add NFRs for:
- Max token budget per report type
- Retry policy (e.g., 3 retries with exponential backoff, then fail + alert)
- Fallback: queue for retry vs. skip vs. degrade to template-based summary
- Cost monitoring/alerting

---

### P1-2. "内存遍历 + 正则匹配" Search Has No Migration Path to v2 Semantic Search

**Finding**: FR-9 specifies "内存遍历 + 正则匹配（短期方案）" as the search implementation. The PRD explicitly defers "语义搜索" to v2 (§5 Non-Goals), but provides:
- No data migration plan from regex-based indices to embedding-based indices
- No schema versioning for search results
- No deprecation timeline for the short-term solution
- No criteria for when v2 should be prioritized

**Impact**: Teams often never migrate "temporary" solutions. Without a migration path, the short-term solution becomes permanent technical debt.

**Recommendation**: Add a "Migration Path" subsection to §6.2 or §11.3 that defines:
- What data needs to be preserved/reindexed for v2
- Whether the regex search API needs to be versioned
- Trigger condition for v2 prioritization (e.g., "when average search latency > 5s for >50% of users")

---

### P1-3. Consequences Lack Acceptance Criteria — "系统应..." Without "怎么算对"

**Finding**: Multiple FRs describe what the system "should" do but do not define what "correct" looks like. Examples:

| FR | Vague Consequence | Missing Acceptance Criteria |
|----|-------------------|----------------------------|
| FR-1 | "日报包含：对话摘要、关键决策、完成的任务、待跟进事项" | How is "关键决策" defined? Who validates correctness? What if LLM hallucinates? |
| FR-1 | "日报生成失败时，记录错误日志，不阻塞其他功能" | What log level? Where are logs stored? What is the alerting threshold? |
| FR-6 | "SOP 包含：目的、步骤、注意事项、参考链接" | What if LLM-generated SOP is nonsensical? Is there human review? |
| FR-8 | "支持去重：已存在的记忆不重复写入" | How is "duplicate" defined? Exact match? Semantic similarity? |
| FR-9 | "搜索方式：内存遍历 + 正则匹配" | Regex on what encoding? UTF-8 only? Case sensitivity? Unicode normalization? |

**Impact**: QA cannot write test cases. Developers cannot know when they're done. PM cannot verify delivery.

**Recommendation**: For each FR, add at least one concrete acceptance criterion that can be objectively verified (e.g., "Given a session with 3 messages containing 'database optimization', searching for 'database optim' returns all 3 messages within 2s").

---

## P2 — Medium Risk (Address in v1 or Early v2)

### P2-1. ASSUMPTION-4: "性能瓶颈在可接受范围内" is Circular and Unfalsifiable

**Finding**: ASSUMPTION-4 states: "历史搜索的性能瓶颈在可接受范围内（短期方案）".

**Problems**:
- "可接受" is undefined. Acceptable to whom? Under what conditions?
- The Success Metric SM-3 says "< 5s for 100 sessions", but the assumption claims this is already acceptable without evidence.
- No benchmark or load test is referenced.

**Recommendation**: Replace with a measurable assumption: "假设 100 个 session 的内存正则搜索可在 5s 内完成（待验证）". Add a validation task to the MVP.

---

### P2-2. ASSUMPTION-3: "用户对报表内容的质量要求以'可用'为标准" is Dangerous

**Finding**: ASSUMPTION-3 assumes users will accept "good enough" report quality.

**Problems**:
- "可用" is subjective and undefined.
- If LLM hallucinates or misses critical decisions, the user experience degrades rapidly.
- No mechanism is defined for user feedback on report quality.

**Recommendation**: Define "可用" with at least one proxy metric (e.g., "user does not delete the report within 24 hours" or "user clicks '固化' on at least one section").

---

### P2-3. Report Storage Path Collisions Possible

**Finding**: FR-1, FR-2, FR-3 define report storage paths using date formats that could collide:
- Daily: `{YYYY-MM-DD}.md`
- Weekly: `{YYYY-Www}.md` — ISO week format, but what if the system locale differs?
- Monthly: `{YYYY-MM}.md`

**Problems**:
- No mention of timezone handling. Is the date UTC? Local time? User-configured?
- What happens if a report already exists? FR-4 mentions "强制刷新" but FR-1-3 do not mention collision handling.
- No file locking mechanism is described for concurrent access.

**Recommendation**: Specify timezone (UTC recommended), define collision behavior (overwrite vs. version vs. error), and document file locking strategy.

---

### P2-4. Missing: Report Deletion / Lifecycle Management

**Finding**: The PRD describes report generation and viewing, but never mentions:
- Can users delete reports?
- Is there automatic archival or cleanup of old reports?
- What is the retention policy?

**Impact**: Disk space will grow unbounded. Users may be surprised by accumulating files.

**Recommendation**: Add a lifecycle management section or at minimum a note that retention policy is TBD.

---

### P2-5. FR-10 "不要求可视化展示" is a Cop-Out

**Finding**: FR-10 states search results "不要求可视化展示（可通过 API/命令行返回）".

**Problems**:
- The PRD is for a GUI application (Tauri). Saying "no visualization needed" ignores the actual user journey.
- If results are returned as "文本列表或结构化 JSON", how does the GUI display them?
- This creates an implicit requirement on the GUI team to build visualization later, without scoping it.

**Recommendation**: Either define the minimum viable display format (e.g., "collapsible list with session link") or explicitly move GUI visualization to Out of Scope with a v2 ticket.

---

## Summary Table

| ID | Severity | Finding | Status |
|----|----------|---------|--------|
| P0-1 | Blocking | Decision Log vs PRD contradiction on "Confirmed" vs "Open" | **UNRESOLVED** |
| P0-2 | Blocking | "GUI 已就绪" claim unverified | **UNRESOLVED** |
| P0-3 | Blocking | Session atomic write cherry-pick lacks risk assessment | **UNRESOLVED** |
| P1-1 | High | No LLM token cost budget or failure retry strategy | **UNRESOLVED** |
| P1-2 | High | No migration path from regex search to semantic search | **UNRESOLVED** |
| P1-3 | High | FR Consequences lack acceptance criteria | **UNRESOLVED** |
| P2-1 | Medium | ASSUMPTION-4 is circular/unfalsifiable | **UNRESOLVED** |
| P2-2 | Medium | ASSUMPTION-3 is subjective and untested | **UNRESOLVED** |
| P2-3 | Medium | Report storage path timezone/collision undefined | **UNRESOLVED** |
| P2-4 | Medium | No report lifecycle/retention policy | **UNRESOLVED** |
| P2-5 | Medium | FR-10 defers visualization without scoping | **UNRESOLVED** |

---

## Recommendations for Next Steps

1. **Immediate**: Hold a PRD review meeting with the decision log owner to reconcile P0-1.
2. **Short-term**: Add acceptance criteria to all FRs (P1-3). This is the single highest-leverage improvement.
3. **Before implementation start**: Produce a one-page risk assessment for the session atomic write cherry-pick (P0-3).
4. **Before release**: Define LLM cost controls and retry strategy (P1-1). This is a production readiness blocker.
