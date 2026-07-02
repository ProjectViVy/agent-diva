# PRD Quality Review v2 — Agent-Diva Pro 报表系统 & Session 历史检索

> Re-validation after P0 fixes (2026-06-08). Previous review: `review-rubric.md`.

---

## Overall verdict
**Adequate with reservations.** The three P0 blockers have been adequately resolved. The PRD no longer contains internal contradictions, the GUI readiness claim is now backed by a verifiable commit hash, and the Session atomic-write fix is described with sufficient concreteness (broken behavior → fix mechanism → failure mode → code reference). However, the document still carries a large backlog of High and Medium findings from the original review—most notably the absence of acceptance criteria for every FR, undefined output schemas for reports and search results, and missing LLM/cost trade-off analysis. For internal-tool stakes, the document is now **safe to prototype from** but remains **under-specified for production implementation**.

---

## Dimension verdicts

| Dimension | v1 Verdict | v2 Verdict | Δ |
|-----------|-----------|-----------|---|
| Decision-readiness | thin | thin | → |
| Substance over theater | adequate | adequate | → |
| Strategic coherence | adequate | adequate | ↑ (P0-3 fix clarifies dependency) |
| Done-ness clarity | thin | thin | → |
| Scope honesty | adequate | **adequate** | ↑ (P0-1 fix resolves state inconsistency) |
| Downstream usability | adequate | adequate | → |
| Shape fit | adequate | adequate | → |

---

## P0 Fix Validation

### P0-1: Decision Log vs PRD contradiction — FIXED ✅
- **Evidence**: Decision Log Open Items table (lines 20–24) now marks all 5 items as `Closed`.
- **Evidence**: PRD §8 Open Questions (lines 274–278) now shows all items with strikethrough and `已确认`.
- **Assessment**: Contradiction eliminated. Single source of truth established. No regression introduced.

### P0-2: "GUI 已就绪" unverified — FIXED ✅
- **Evidence**: PRD §6.1 In Scope (line 241) now reads: `GUI 已就绪，commit fcf768d，NotebookView.vue 725 行`.
- **Assessment**: The claim is now falsifiable (verifiable against commit `fcf768d`). Line count provides additional specificity. No regression introduced.

### P0-3: Session atomic write insufficiently assessed — FIXED ✅
- **Evidence**: PRD §6.1 In Scope (line 244) now reads:
  > Session 原子写入修复（当前 `SessionManager::save` 为非原子写入，直接覆写 `.jsonl` 文件；MVP 需改为临时文件写入后 rename，避免进程崩溃导致 session 数据丢失。见 `agent-diva-core/src/session/manager.rs:124`）
- **Assessment**: Now contains all four required elements: (1) current broken behavior, (2) concrete fix mechanism (temp file + rename), (3) specific failure mode (crash → data loss), (4) code reference. No regression introduced.

---

## Remaining Findings (by severity)

### Critical (1)
- **C-1** All FRs still lack acceptance criteria (§4.1–§4.3) — *carried from v1 critical finding*. Consequences describe behavior but provide no testable boundary. Engineers cannot determine "done." *Fix: Add Given-When-Then or checklist-style acceptance criteria to each FR.*

### High (4)
- **H-1** Missing LLM selection & cost trade-off analysis (§7, §11.3) — *carried from v1*. No model specified, no token budget, no cost guardrail. *Fix: Add decision matrix (model / cost / quality / latency) and cost ceiling.*
- **H-2** "Agent 智能搜索" implementation path lacks technical comparison (§4.3 FR-9) — *carried from v1*. Only "内存遍历 + 正则匹配（短期方案）" is stated without rationale for rejecting SQLite FTS, grep, etc. No exit condition for "短期." *Fix: Add search roadmap (short-term regex → mid-term SQLite FTS → long-term semantic) with transition triggers.*
- **H-3** Report output schema undefined (§4.1 FR-1/2/3) — *carried from v1*. "Markdown 文件" is insufficient; no template, header structure, or mandatory fields. *Fix: Provide Markdown template with frontmatter schema for daily/weekly/monthly reports.*
- **H-4** "固化 Skill" lacks concrete format reference (§4.2 FR-7) — *carried from v1*. "参考 GenericAgent skill 规范（见 `.workspace/` 目录）" is not actionable. *Fix: Add specific file path or inline minimal example.*

### Medium (9)
- **M-1** Report quality success metric is ambiguous (§7 SM-1) — *carried from v1*. "成功率 > 95%" lacks operational definition. *Fix: Define "success" (e.g., no exception exit + all 4 required sections present).*
- **M-2** NFR section is ceremonial (§10) — *carried from v1*. Principles listed without verification method or FR traceability. *Fix: Add verification method (test case or metric) and traceability link per NFR item.*
- **M-3** "Agent 智能搜索" naming is innovation theater (§4.3, Glossary) — *carried from v1*. "智能" oversells regex traversal. *Fix: Rename to "Session 关键词搜索" or "历史对话检索"; reserve "智能搜索" for v2 semantic search.*
- **M-4** Report solidification dependency on report generation is unstated (§4.2) — *carried from v1*. FR-6/7/8 assume reports exist but do not define behavior when generation fails or output is empty. *Fix: Add precondition in FR-6/7/8 Consequences: "仅当关联报表已成功生成且内容非空时，固化功能可用."*
- **M-5** Session search and report generation synergy is unstated (§4.1 vs §4.3) — *carried from v1*. Both consume session history but interaction is undefined. *Fix: Explicitly state whether they are independent or if search results feed into reports.*
- **M-6** "固化为 SOP" lacks concrete format reference (§4.2 FR-6) — *carried from v1*. Same pattern as H-4. *Fix: Add specific file path or inline minimal example.*
- **M-7** Search result JSON schema undefined (§4.3 FR-10) — *carried from v1*. "文本列表或结构化 JSON" is too vague. *Fix: Provide JSON schema with fields: session_key, timestamp, message_content, match_type, etc.*
- **M-8** Term drift: "Report / 报表 / 报告" used inconsistently (全文) — *carried from v1*. Glossary defines "Report / 报表" but "报告" appears in §1 Vision and elsewhere. *Fix: Standardize on Glossary term.*
- **M-9** Missing UJ-to-FR cross-reference matrix — *carried from v1*. "Realizes UJ-N" exists but no reverse lookup. *Fix: Add mapping table at §4 Features header or §2.3 end.*

### Low (7)
- **L-1** Vision paragraph is slightly grandiose (§1) — *carried from v1*. "进化为具备自我回顾、知识沉淀、历史检索能力的智能体" oversells internal-tool stakes. *Fix: Tighten to user-value framing (e.g., "减少用户重复回顾 session 的时间成本").* — **Non-blocking, cosmetic.**
- **L-2** Unstated assumption: workspace directory is writable (§4.1, §4.2) — *carried from v1*. All storage paths assume `{workspace}` is writable but not listed in Assumptions. *Fix: Add ASSUMPTION-5.* — **Non-blocking, easily discovered during implementation.**
- **L-3** "Non-Users (v1)" scope for "配置关闭" is ambiguous (§2.2) — *carried from v1*. Unclear if toggle is in MVP. *Fix: Clarify in §6.1 or §6.2.* — **Non-blocking, can be resolved during sprint planning.**
- **L-4** Missing report generation retry policy — *carried from v1*. FR-1 says "记录错误日志" but no retry / backoff / alert policy. *Fix: Add explicit statement: "不重试，仅记录日志并告警" or define retry strategy.* — **Non-blocking, operational detail.**
- **L-5** GUI "双栏布局" lacks interaction details (§4.1 FR-5) — *carried from v1*. No responsive behavior, empty state, loading state. *Fix: Reference existing GUI design doc or add state descriptions.* — **Non-blocking, GUI team can fill gap.**
- **L-6** References file paths not verified (§12) — *carried from v1*. `agent-diva-gui/src/components/NotebookView.vue` and `.workspace/genericagent/` paths cannot be verified in review environment. *Fix: Pre-submit path validation or add disclaimer.* — **Non-blocking, mechanical.**
- **L-7** Glossary "Session 任务" semantic overlap with FR-9 — *carried from v1*. Search task is a background async task but not explicitly declared as a "Session 任务" instance. *Fix: Clarify in FR-9 or adjust terminology.* — **Non-blocking, terminological.**
- **L-8** Success metric SM-2 is product-oriented for internal tool (§7) — *carried from v1*. "用户每周至少查看 1 次报表" is engagement metric; internal tools need efficiency metric. *Fix: Rephrase to "开发者平均每周节省 X 分钟用于手动回顾 session".* — **Non-blocking, metric preference.**
- **L-9** "固化为 SOP/Skill/Memory" spans too much complexity for one PRD (§4.2) — *carried from v1*. Knowledge management design approaches independent sub-system scope. *Fix: Consider splitting §4.2 into separate PRD or add architecture overview.* — **Non-blocking, structural preference.**

---

## Finding counts by severity

| Severity | Count | v1 Count | Δ |
|----------|-------|----------|---|
| Critical | 1 | 1 | → |
| High | 4 | 4 | → |
| Medium | 9 | 10 | ↓ 1 (P0-1 state-inconsistency resolved) |
| Low | 9 | 9 | → |
| **Total** | **23** | **24** | **↓ 1** |

---

## New issues introduced by P0 fixes

**None.** The three fixes are surgical and do not introduce regressions, contradictions, or ambiguities.

---

## Recommendations (prioritized)

1. **Before implementation starts**: Add acceptance criteria to all 10 FRs (C-1). This is the single biggest blocker to engineer productivity.
2. **Before implementation starts**: Define report Markdown template / schema (H-3) and search result JSON schema (M-7). These are prerequisites for API contract and GUI integration.
3. **Before sprint 1 planning**: Add LLM decision matrix (H-1) and search roadmap (H-2). These affect architecture and cost.
4. **During implementation**: Resolve term drift (M-8), add UJ-FR matrix (M-9), and solidify format references (H-4, M-6). These are quick wins that improve downstream usability.
5. **Nice to have**: Tighten Vision rhetoric (L-1), adjust Success Metrics (L-8), and consider splitting §4.2 (L-9). These are cosmetic / structural improvements that can be deferred.

---

## Mechanical notes (unchanged from v1)

- **ID continuity**: FR-1–FR-10 continuous; UJ-1–UJ-2 continuous; SM-1–SM-C1 continuous; ASSUMPTION-1–ASSUMPTION-4 continuous. ✅
- **Section separators**: Document still uses `---` as section dividers, which conflicts with Markdown horizontal-rule syntax. Minor rendering risk in some parsers. — **Non-blocking.**
- **Decision Log → PRD traceability**: Change #6 and #7 in Decision Log correctly map to P0-2/P0-3 and P0-1 fixes respectively. ✅
