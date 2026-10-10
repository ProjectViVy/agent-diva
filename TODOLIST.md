# TODOLIST

## Current attention — 2026-10-10

Only current DIVA, VIVY, and Laputa cross-project watch items are listed here.
The previous 50-item deferred backlog is preserved in [`defer.md`](defer.md).
Deferred means postponed, not completed or accepted.

- [ ] **DEFERRED — VIVY PR #46 / Laputa PR #5 sequencing** — Both PRs are open and mergeable; commits resolve to `mastwet`. VIVY CI run #77 failed because its bootstrapped Laputa dependency lacks APIs introduced in PR #5. After PR #5 is integrated or the dependency pin is updated, rerun all VIVY CI lanes and confirm the aggregate `just ci` gate.
- [ ] **DEFERRED — DIVA PR #21 author attribution** — PR #21 is merged and was created by `mastwet`, but its commits resolve to `mas19192` via `0104988com@gmail.com`. Leave shared `main` unchanged; revisit only if the owner explicitly decides to correct merged history.
