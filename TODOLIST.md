# TODOLIST

Updated: 2026-10-10. This is the current DIVA/VIVY/Laputa follow-up list. The 47 non-current deferred backlog entries are archived in [`defer.md`](defer.md).

- [ ] **Laputa PR #5 / VIVY PR #46 review and dependency sequence** — Both PRs remain open and mergeable; their commits resolve to `mastwet`. VIVY needs APIs introduced in Laputa PR #5. Track review and integration order.
- [ ] **VIVY CI run #77** — It failed against the bootstrapped Laputa dependency, which lacks APIs from PR #5. After PR #5 is integrated or VIVY's dependency pin is updated, rerun all VIVY CI lanes and confirm `just ci` passes.
- [ ] **DIVA PR #21 author attribution** — The merged PR was created by `mastwet`, but its commits resolve to `mas19192` via `0104988com@gmail.com`. Attribution remains unresolved; only revisit correction after an explicit decision to rewrite shared `main`.
