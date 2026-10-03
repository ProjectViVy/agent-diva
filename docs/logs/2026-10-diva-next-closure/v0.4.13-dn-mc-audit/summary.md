# v0.4.13 — DN-M-C boundary + behavior audit

Reconciled the closure ledger, boundaries and dependencies against the
integrated candidate and produced the scoped R-1…R-9 matrix that DN-8C
consumes.

## Evidence

- TODOLIST reconciled: eight closure rows marked landed with evidence
  pointers; cancelled DN-7 import kept separate (no importer exists —
  verified by source scan; OBS-09 exercised a fresh-born home).
- Gates: `check_legacy_frontend_calls.mjs --selftest` 9/9 after adding
  the missing seam-internal unlisted-command fixture;
  `check_vivy_backend_boundary.py` clean (485-package graph, no legacy
  crates, 13-command allowlist, no sidecars).
- Dependency audit: `cargo audit` 0.22.2 (RustSec 1290 advisories) —
  src-tauri lock 0 vulns / 2 warnings (proc-macro-error unmaintained;
  glib unsound VariantStrIter, Linux-GTK-only); vivy-bridge lock clean.
  `pnpm audit --prod` — 8 advisories, all transitive/dev-facing;
  dispositions stay open in TODOLIST.
- Scoped checks: vitest 67/551, cargo -p vivy-bridge -p diva-speech 25
  green.
- C2-8 matrix appended to the contract ledger; index updated; 10 exact
  rows passed to DN-8C.

## Deferred

- Everything DN-8C inherits: OBS09-F1..F4 dispositions, live child run,
  window hide/reopen, log rotation, recovery re-drive, live-provider
  speech gates, whole-workspace compile, Windows x64 packaging,
  owner acceptance.

Owner acceptance: pending.
