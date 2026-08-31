# Risks, rollback, and C6 boundary

## Principal risks and required controls

| Risk | Control/stop condition |
| --- | --- |
| Octos copied as a framework | Port only listed protocol behavior; reject ChannelManager/bus/default no-ops |
| Apache-2.0 provenance lost | Ledger every non-trivial adaptation with SHA, symbol, destination, and test |
| MSRV/dependency regression | Existing transports first; Rust 1.80 probe blocks merge |
| Listener invisible to supervisor | `start` remains long-running; no detached spawn-and-return |
| ACK before bounded admission | Commit dedup/ACK only after admission or force platform retry |
| Cross-chat/thread leakage | Address/correlation frozen per command; no sticky current chat/thread |
| Silent media loss | Unsupported or partial failure is typed; no log-and-continue success |
| False delivery claims | Default successful platform API state is `Accepted`, not `Delivered` |
| Blocking Email stalls runtime | Bounded `spawn_blocking`, cancellation fencing, fake IMAP/SMTP tests |
| Global test races | Private endpoint injection; no mutable process-wide override variables |
| Secret/PII leakage | Redacted fixtures/logs, external live credentials, hash/truncate identifiers |
| Parallel agent conflicts | Shared contract first, isolated worktrees, predeclared non-overlapping locks |

## Rollback

C5 is not deployed. Roll back by reverting the focused platform or shared-contract commit on
`feat/channel-epic`; never use destructive reset or remove unrelated work. A platform commit is not
accepted if its rollback would require reverting another platform. Retain fixture and verification
records that explain why a capability was withdrawn.

If a shared seam proves wrong after workers branch, stop integration, resolve the ADR, commit one
shared correction, then rebase/cherry-pick workers. Do not layer a compatibility wrapper over the
wrong seam.

## Explicit C6-only work

- Manager production construction of `AdapterServices`, Registry, pacing, and supervisors.
- Replacement/removal of `ChannelManager`, `ChannelHandler`, old message DTO/bus, legacy real-time
  paths, Neuro-Link handler/config aliases, and retired channel source/features.
- GUI/config cleanup that depends on deleted legacy surfaces.
- Clean-break `rg` gate, complete workspace/GUI/Tauri/MSRV gates, real desktop disconnect recovery,
  and one atomic merge into `dev`.

C5 must leave enough factory/services surface for C6, but it must not activate a second production
path or merge partial dual-stack state into `dev`.

## Completion truth table

- Documents frozen, code not started: C5-P complete; C5 remains open.
- Six adapters and offline evidence pass, QQ credentials unavailable: C5-I complete, C5-V open.
- QQ live smoke and all gates pass: C5 complete on the isolated branch; still no `dev` merge.
- C6 clean break plus desktop acceptance pass: eligible for atomic merge and release acceptance.
