# Release — OBS-08

Commit: `feat(diagnostics): show logs and persistence loss` on
`feat/dn-closure-wave1`. No push, no PR, no release artifacts.

New/changed:
- `state/gui-diagnostics.ts` (new): recorder + reader + entry point.
- `components/console/DiagnosticsPanel.vue` (new) + ConsoleView mount.
- `api/vivy/{contracts,client,observability}.ts`: diagnostics types/calls.
- locales en/zh: `diagnostics.*` keys.

Boundaries: latest-page memory only; queue is best-effort (loss counted,
not hidden); no durable GUI log DB, no browser path input.
