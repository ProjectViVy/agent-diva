# Shared decision requests

Implementation begins only while this file has no open request.

## Open

None.

## C5-V review (2026-09-02)

No new shared-contract decision request was opened. The local wire evidence is confined to
adapter-owned transports and fixtures. QQ D-013 and D-014 remain the explicit blocked decisions;
their resolution still requires official delivery/media evidence and cannot be replaced by a
local mock or an inferred endpoint.

## C5-V repair disposition (2026-09-02)

The 21 audited `partial` rows were repaired in six isolated channel lanes and rechecked against
Octos `5ea987813de4fd2afdd1d78f2106ad2868f0d923`. No shared contract or configuration seam was
opened. The resulting local disposition is `11 verified / 17 partial / 1 blocked/unsupported`.
Telegram keyboard/reply markup stays unsupported because the public command contract is frozen;
QQ D-013/D-014 and the ignored live harness remain external blockers. No shim, private side
channel, global endpoint override, or Manager/C6 change is authorized by this record.

## Request format

- Status: `open` or `resolved`
- Requester/worktree:
- Platform and blocked capability:
- Frozen contract that appears insufficient:
- Evidence: exact platform/Octos/Diva file, symbol, fixture, or error
- Minimal proposed shared change:
- Alternatives rejected:
- Lead decision, rationale, and commit:

Workers must stop at the blocked seam. They may continue unrelated work in their owned platform
files but may not edit shared contracts, Cargo, `lib.rs`, global tests, or another worker's scope.
