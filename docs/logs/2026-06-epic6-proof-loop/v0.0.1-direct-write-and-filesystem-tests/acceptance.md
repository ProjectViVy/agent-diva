# Acceptance

## Product Checks

- Run `just epic6-proof-check` and verify the proof suite completes without starting the GUI.
- Introduce a temporary forbidden direct write to a legacy authority file outside `agent-diva-laputa` and verify `authority_boundaries` fails with an actionable file/location message.
- Introduce a temporary direct runtime read of a legacy authority file in a guarded runtime crate and verify `authority_boundaries` fails with an actionable file/location message.

## Technical Checks

- Confirm the authority boundary allowlist is limited to approved filesystem implementations and the explicit bootstrap compatibility read.
- Confirm Laputa storage tests cover atomic writes, timeout behavior, stale-lock recovery, and a Windows-safe workspace path.
- Confirm Laputa migration tests cover stale staging cleanup before commit.
