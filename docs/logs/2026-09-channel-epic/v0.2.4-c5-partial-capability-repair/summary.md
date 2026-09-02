# C5 Partial Capability Repair

## Scope

This iteration repairs the 21 capability rows that remained `partial` after the C5-V source audit. Work is based on the pinned Octos checkout at `5ea987813de4fd2afdd1d78f2106ad2868f0d923` and remains isolated on `feat/channel-epic`.

The implementation scope is limited to native channel adapters, deterministic wire/lifecycle fixtures, channel tests, shared evidence checks, and the corresponding Gate3 records. `ChannelCommand`, `ContentPart`, configuration keys, Manager assembly, and the `dev` branch remain unchanged.

## Boundaries

- Telegram keyboard/reply markup remains `UnsupportedCapability`; callback acknowledgement may be repaired and evidenced without adding a public keyboard contract.
- QQ D-013 official event-delivery proof, D-014 official media proof, and the real-credential vertical smoke remain external gates.
- No production Manager/C6 cutover, legacy handler deletion, `dev` merge, or push is part of this iteration.

## Status

Implementation and evidence integration are in progress. Rows are promoted to `verified` only after source symbol, fixture, test, exact request/response or typed error, receipt, and lifecycle evidence all agree.
