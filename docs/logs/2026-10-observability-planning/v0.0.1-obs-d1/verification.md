# Plan verification

Scope is documentation only. Validation covers local links/anchors, required Story sections and checkboxes, requirement ownership, central state consistency, known immediate predecessors, DAG cycles/transitive duplicates, existing-source path/signature evidence and scoped whitespace checks. Independent read-only source investigation covers DIVA actual/proposed APIs and pinned Eino/provider observation limitations. Review findings are reconciled before commit.

Commands executed in the isolated planning worktree:

- `python /workspace/scratch/91cf03f9a589/checks/validate_obs_plan.py` — exit 0. Audit reads 19 Markdown documents; verifies 89 local link/anchor occurrences, 22 central Story nodes, 29 dependency edges, nine executable OBS plans with 59 checkbox steps, O1–O8 coverage, Planned/Blocked consistency, and 22 existing inspected source paths plus three actual signatures. No new errors, unknown endpoints, cycles, self-dependencies or transitive duplicate edges.
- `git diff --check` — exit 0 before staging. `git diff --cached --check` initially found trailing blank lines in the new Story files; normalized EOFs, restaged, then reran the staged check successfully before commit.

The first audit caught missing explicit red/green markers in OBS-09, which were added. It also found 21 inherited broken-link occurrences in TODOLIST.md; these occur unchanged at base 3407b3c and are recorded under its existing GOVERNANCE-DOCS-STALE item. They are not silently counted as new-link passes.

Pinned-source review changed the draft to cover BaseModel summary primary/fallback routes, a post-success StreamOpened marker, actual common tool options, model budget admission and observed-call IDs only. Independent plan review identified separate call/run state and optional usage-presence ambiguity; D2/D4 and consumer plans now explicitly preserve unknown bucket presence and independent run_activity/call_status, with legacy wire fields kept for compatibility. Product Go/GUI/Cargo/native suites are not rerun for this documentation change; their execution commands and failure/acceptance oracles are in each Story. Native bridge sources and an accepted packaged artifact are not available, so no build/runtime acceptance is claimed.

Final source follow-up: Claude conversion discards delta field presence; Eino itself uses per-bucket max usage merging. D2 now separates bounded provider-stream normalization from Journal latest-sample replacement, derives normalized total from input+output and tests output-only delta/final usage-only chunks. Ambiguous normalization remains partial, not invoice/exact-final evidence.

Independent review's remaining Important MaxEvents issue is resolved with explicit bounded mandatory settlement semantics and replay/boundary tests; its Minor coverage invariant issue is resolved by adding completed_with_usage. Follow-up reviewer confirmed both fixes, with no remaining material findings. Product code remains unchanged.
