# C5 Partial Capability Repair

## Scope

This iteration repairs the 21 capability rows that remained `partial` after the C5-V source audit. Work is based on the pinned Octos checkout at `5ea987813de4fd2afdd1d78f2106ad2868f0d923` and remains isolated on `feat/channel-epic`.

The implementation scope is limited to native channel adapters, deterministic wire/lifecycle fixtures, channel tests, shared evidence checks, and the corresponding Gate3 records. `ChannelCommand`, `ContentPart`, configuration keys, Manager assembly, and the `dev` branch remain unchanged.

## Boundaries

- Telegram keyboard/reply markup remains `UnsupportedCapability`; callback acknowledgement may be repaired and evidenced without adding a public keyboard contract.
- QQ D-013 official event-delivery proof, D-014 official media proof, and the real-credential vertical smoke remain external gates.
- No production Manager/C6 cutover, legacy handler deletion, `dev` merge, or push is part of this iteration.

## Status

The six repair lanes and Lead shared TCK are implemented and integrated. The machine-auditable
disposition is `11 verified / 17 partial / 1 blocked/unsupported` across 29 rows. Feishu FS-01,
FS-03, FS-04, and FS-05 were promoted only after deterministic wire/store/lifecycle evidence
closed their local requirements; all other repaired rows remain conservative where live platform,
supervisor, or external decision evidence is still absent.

The repair commits are `d22f82c1` (QQ), `17c3749c` (DingTalk), `e0ff3f9a` (Feishu), `981a4884`
(Telegram), `b82b80c5` (Email), `760564b1` (Discord), `73c0359c` (shared TCK), `f6cedfdb`
(typed Email MIME error), `9579e577` (all-target clippy cleanup), and `6c06a069` (QQ live
prerequisite wording). The branch remains isolated; C5-V/C5-Q are not closed because QQ live,
D-013/D-014, Telegram keyboard boundary, and the remaining external evidence gates remain open.
