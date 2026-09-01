# Shared decision requests

Implementation begins only while this file has no open request.

## Open

None.

## C5-V review (2026-09-02)

No new shared-contract decision request was opened. The local wire evidence is confined to
adapter-owned transports and fixtures. QQ D-013 and D-014 remain the explicit blocked decisions;
their resolution still requires official delivery/media evidence and cannot be replaced by a
local mock or an inferred endpoint.

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
