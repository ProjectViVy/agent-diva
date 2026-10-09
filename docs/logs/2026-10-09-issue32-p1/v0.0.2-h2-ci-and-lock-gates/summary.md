# P1.2 H2 CI and lock gates

Date: 2026-10-09
Repository: `ProjectViVy/agent-diva`
Branch: `feat/issue32-desktop-gates`
Scope: Issue #32 P1.2 active Go-host triggers, platform locks, CI aggregate,
and desktop boundary checks.

## Delivered

- Extended push and pull-request path filters to cover the Go host, desktop and
  speech packages, source locks, scripts, generated bindings, and aggregate
  recipe.
- Added Linux and Windows native jobs. Both check out VIVY and Laputa at the
  canonical source-lock commits and read Go, Node, pnpm, and Wails versions
  from locked inputs.
- Added a required workflow job that fails unless the Linux aggregate, Windows
  Go-host race job, VIVY transition guard, and Windows shell job pass. The Linux
  aggregate runs GUI tests/build, Go race tests, binding drift, shell bridge
  tests, both boundary guards, legacy guard, and sealed build/inspection.
- Replaced the single-target lock with one source lock declaring Linux and
  Windows target settings. Added deterministic target derivation, canonical
  JSON hashing, source/tool checks, generated-lock drift checks, and retained
  source and target locks in the sealed artifact checksum set.
- Upgraded the inofy source pin to its full commit and pinned VIVY to the
  committed SDK source-closure repair (`9ba59caa4816a7f66e27c50b7948ddeb09a2df9c`).
- Added a boundary scanner for VIVY internal imports, direct native frontend
  calls, browser provider fetches, and packaged sidecars/second runtimes.
- Added optional focused Go package and test-name arguments to test mode.

The existing Tauri/Rust transition guard remains in the workflow.

## Acceptance state

The H2 implementation and local contract checks are complete. The workflow has
not run on GitHub in this local-only session. Linux and Windows native Go race
acceptance therefore remains pending; H2 is not yet engineering-verified.
No push, publication, or release was performed.
