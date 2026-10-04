# DIVA Next — authoritative delivery index

Revision **DN-W3-P1**, 2026-10-04. This is the single live migration status
and dependency authority. [Architecture](p0-design.md) and
[contracts](backend-separation-contracts.md) remain in their existing locations.
[Archive record](wails/archive.md) fixes the source baseline.
The owner approved Wails/Go migration direction and this planning/archive
publication. Product implementation and mainline promotion are a later phase.

## Decision and completion boundary

Replace the Tauri/Rust host and custom Go C ABI with one Go desktop host using
Wails. Retain the Vue/TypeScript/VRM frontend, VIVY runtime, Journal, policy,
approvals, cognitive composition and existing domain APIs. Speech moves from
DIVA Rust to DIVA Go. VIVY remains independently usable without Wails.

This publication changes documentation only. It does not delete production
code, merge either main branch, publish a product release, claim Windows
acceptance, or revive the old Rust business backend. Source archive branches
exist at the paired SHAs below; paired tags are pending the tag-write access
described in the archive record.

| Repository | Frozen main commit | Archive branch | Documentation branch |
| --- | --- | --- | --- |
| agent-diva | `5444795a2d9db31e158c2cf009d64697e6289e50` | `archive/tauri-cabi` | `docs/wails-migration-20261004` |
| agent-vivy | `fc559e6b03ce4e65c0099b9745855dccc4fb067e` | `archive/tauri-cabi` | `docs/wails-migration-20261004` |

The freeze includes merged closure work, including native speech. It excludes
unmerged branches such as DIVA `fix/windows-speech-shell`; inspect relevant
fixes before implementation, without treating them as archived mainline facts.
Do not reset either archive ref as main advances.

## Requirements

| ID | Required outcome | Delivery evidence |
| --- | --- | --- |
| W-R1 | One Go owner for desktop/runtime lifetime; no application C ABI or Rust host | W1/W3/W6 races, teardown, dependency scan |
| W-R2 | Sealed Generation, one Runtime/Journal, enforced policy and ActionHost grants | W1/W2/W5 positive and fail-closed tests |
| W-R3 | Existing chat, images, permissions, approvals, cancellation and cognitive/console UI preserved | W3/W5 scenario matrix |
| W-R4 | SiliconFlow STT and SiliconFlow/MiniMax TTS, native secrets/assets, no stale playback | W0/W4/W5 native and provider evidence |
| W-R5 | Hide/reopen continuity, explicit quit semantics, bounded and truthful shutdown | W1/W3/W5 teardown evidence |
| W-R6 | Independent reproducible source-locked native build and useful diagnostics | W2/W5/W7 manifest/hash/clean machine evidence |
| W-R7 | Complete C ABI/Tauri retirement on main after cutover; recoverable old source line | Archive + W6 inventory |
| W-R8 | Windows x64 installed-product acceptance prepared by engineering, signed off by owner | W7 acceptance record |
| W-R9 | Fresh Next state; no historical import, old-home access, dual writes or deletion | W3/W5 clean-profile evidence |

## Executable Story package

Every Story below has its own files, shared-contract references, ordered tasks,
verification commands and exit gate. **Planned** means implementation has not
started or been authorized in this publication. **Blocked** means a concrete
predecessor must deliver evidence first. There are no Ready or Done
implementation stories merely because this plan exists.

| Story | Outcome | Immediate prerequisites | State | Estimated engineering days |
| --- | --- | --- | --- | --- |
| [W0](wails/W0.md) | Pinned Wails native feasibility and security contract | None | Planned; probe before adoption | 1–2 |
| [W1](wails/W1.md) | Public Go host, single lifecycle, logging/admission baseline | None | Planned | 3–5 |
| [W2](wails/W2.md) | Sealed Go-host package target and dependency closure | W1 | Blocked by W1 | 3–5 |
| [W3](wails/W3.md) | Wails desktop, frontend seam and window lifecycle | W0, W2 | Blocked by W0/W2 | 2–4 |
| [W4](wails/W4.md) | Go speech/credentials/assets and binary media route | W0 | Blocked by W0 | 3–5 |
| [W5](wails/W5.md) | Full integration, recovery and observability closure | W3, W4 | Blocked by W3/W4 | 3–5 |
| [W6](wails/W6.md) | Remove C ABI/Rust and replace CI/build/documentation | W5 | Blocked by W5 | 1–2 |
| [W7](wails/W7.md) | Clean Windows package and final acceptance handoff | W6 | Blocked by W6 | 2–3 |

Estimates are planning ranges, not measurements: **18–31 engineering days**,
roughly 4–7 working weeks for one engineer with native environments available.
Allow another 20% contingency for the new pack target, Wails beta behavior and
keyring/native packaging. Waiting for owner acceptance or platform access is
excluded. Re-estimate after W0/W2; do not use this as a release commitment.

### Topological execution waves

| Wave | Stories | Shared-file coordination |
| --- | --- | --- |
| 1 | W0, W1 | DIVA probe and VIVY host have separate ownership |
| 2 | W2, W4 | Separate repositories; shared contract changes serialized |
| 3 | W3 | DIVA entrypoint, module/lock files and desktop seam have one writer |
| 4 | W5 | Cross-repo integration; update exact paired source pins |
| 5 | W6 | DIVA cutover first; VIVY shared target retirement last |
| 6 | W7 | Native package evidence then owner handoff |

Waves describe dependency eligibility, not permission to spawn agents.
Execution is sequential by default. Assign one writer per shared file:
DIVA Go module/locks, desktop-host.ts, App.vue, CI, and VIVY SDK pack,
embedded host, app composition and recipe. No transitive duplicate edges.

## Mainline promotion and rollback

1. Keep the two archive branches frozen. Complete both matching annotated
   tags before the destructive retirement phase. Record any later legacy
   emergency fix on a distinct branch/tag, never move this baseline.
2. Implement in `feat/wails-migration` branches created after the next execution
   authorization. Reconcile drift from the fixed planning baseline first.
3. VIVY W1/W2 additions can land while current C ABI consumers still build.
   This is a short source transition; no dual-mode shipped DIVA is required.
4. Integrate and validate W3/W4/W5 on DIVA's migration branch. Promote the DIVA
   switch and its Rust deletion together only with passing replacement gates.
5. Retire VIVY C ABI/shared-only code after the DIVA consumer is on the pinned
   Go-host path and other repository consumers have been checked. Preserve
   normal VIVY executable pack/inspect behavior.
6. W7 validates the final resulting pair after W6. A source tag alone is not a
   known-good rollback binary. Reverting a code cutover requires matching
   source pins and a supported state directory; never point the old binary
   at a new database or reset main history.

This order keeps main buildable. The owner-approved target is a clean main
without the custom C ABI, not permanent compatibility maintenance.

## Relationship to prior DN-C2 / OBS work

All old DN/closure/OBS Story files remain historical implementation and
evidence references. Their Tauri, C ABI, shared library, Rust speech and old
Ready/Blocked scheduling instructions are inactive under DN-W3. Do not run
them independently as an alternate migration plan. Their business behavior
and explicitly deferred features still matter.

| Prior scope | DN-W3 disposition |
| --- | --- |
| DN-L, DN-5, DN-LC, DN-P-C, native parts of DN-8C | Replace with W1/W2/W3/W6/W7 |
| DN-1/2/3, DN-2A/B, DN-4*, OBS-06..09 | Preserve delivered consumers and domain authority; verify/correct in W5 |
| DN-0S, DN-6A/B/C | Preserve voice UX/provider behavior; port native implementation in W4 |
| DN-M / DN-M-C | Rebase boundary gates on Go host in W6; behavior closure in W5 |
| DN-7 | Cancelled; no import work or release dependency |
| Local ONNX/OLVRS, pet expansion, broad report/resource systems | Remain deferred, not made prerequisites by migration |

Historical evidence at old artifact pins is not acceptance of the new host.
In particular, closure-packaged-obs.json records 12 passed / 1 failed /
3 pending rows using a local scripted provider. W5 must reproduce relevant
rows and separately run a real configured provider. Fresh FrozenCore
initialization, process-local grants, runtime logs and quit/restart semantics
are explicit W5 gates, not silently closed by changing language.

## Execution contract

Read the architecture and W3 ledger before the selected Story. Confirm
predecessor artifacts and the next execution authorization; use
superpowers:executing-plans task by task. Follow both repositories' AGENTS
and VIVY's module/port/assembly rules. No Agent engine fork, new generic
middleware framework, direct internal imports from DIVA, or hidden feature
expansion. Existing Eino v0.9.13 delegation remains unchanged; desktop hosting
does not introduce new Eino capabilities.

Each implementation Story updates its evidence, index status and owning
backlog. VIVY product-contract changes run its required `just ci`; DIVA
replacement CI must cover Go, frontend and a native package. Unavailable tests
stay pending with an owner/environment, never pass by inference.
