# Deferred follow-ups from the recent PR work

Updated: 2026-10-10. This file contains only the unfinished follow-ups from the recent DIVA/VIVY/Laputa conflict and account-attribution work. The broader DIVA backlog remains in [`TODOLIST.md`](TODOLIST.md).

- [ ] **Laputa PR #5 and VIVY PR #46 — review and dependency order** — Conflict resolution and push are complete; both PRs remain open and mergeable, with commits attributed to `mastwet`. They are awaiting review and owner disposition. VIVY depends on APIs introduced in Laputa PR #5.
- [ ] **VIVY CI run #77 — rerun after dependency update** — The run failed because the bootstrapped Laputa dependency did not contain APIs added in PR #5 (`EvolutionPorts.WithMissionRevision`, capture activity, and bound-client capture/archive methods). Once PR #5 is integrated or VIVY's dependency pin is updated, rerun all CI lanes and confirm `just ci` passes.
- [ ] **DIVA PR #21 — author attribution remains unresolved** — The PR is merged and was created by `mastwet`, but its commits resolve to `mas19192` through `0104988com@gmail.com`. No history rewrite was made. Revisit only if the owner explicitly decides to correct attribution on shared `main`.
