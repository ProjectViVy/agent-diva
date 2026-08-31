# CHANNEL-EPIC C5 Octos capability migration

This package is the decision-complete handoff for C5. It separates the work into a
documentation freeze (`C5-P`), native adapter implementation (`C5-I`), and evidence plus
live validation (`C5-V`). No implementation agent may start from a platform file before
reading the architecture and capability matrix.

## Authorities

- Agent Diva target architecture: [`../architecture.md`](../architecture.md)
- Target branch/worktree: `feat/channel-epic` in
  `C:\Users\Administrator\Desktop\morediva\agent-diva-channel-epic`
- Octos reference snapshot:
  `C:\Users\Administrator\Desktop\morediva\.workspace\octos` at
  `5ea987813de4fd2afdd1d78f2106ad2868f0d923` (`v2.0.3-rc.9`)
- Octos license: Apache-2.0. Record provenance for any non-trivial adapted logic.
- Agent Diva MSRV: Rust 1.80.0. Octos dependencies or Rust-2024-only code are not imported.

## Required reading order

1. [`00-current-state-and-handoff.md`](00-current-state-and-handoff.md)
2. [`01-scan-playbook.md`](01-scan-playbook.md)
3. [`02-octos-provenance-ledger.md`](02-octos-provenance-ledger.md)
4. [`03-capability-gap-matrix.md`](03-capability-gap-matrix.md)
5. [`04-target-architecture-and-adr.md`](04-target-architecture-and-adr.md)
6. The assigned file under [`platforms/`](platforms/)
7. [`05-agent-wbs-and-merge-order.md`](05-agent-wbs-and-merge-order.md)
8. [`06-tck-fixture-and-live-smoke.md`](06-tck-fixture-and-live-smoke.md)
9. [`07-risk-rollback-c6-boundary.md`](07-risk-rollback-c6-boundary.md)

[`decision-requests.md`](decision-requests.md) is an exception queue. It must be empty before
parallel implementation begins. Workers must add a request there instead of editing shared
contracts ad hoc.

## Freeze rule

The matrix in this package is the minimum C5 target. A capability may become `true` only when a
real request/response path and an offline fixture or mock test are both identified. Unsupported
operations return `AdapterError::UnsupportedCapability`; method presence or a silent no-op is not
evidence. QQ group/C2C, typed media, message identity, resume/dedup, and real receipts are mandatory
and cannot be removed to make the gate pass.

## C5/C6 boundary

C5 creates six native `ChannelAdapter` implementations and their proof but does not switch the
production Manager, delete `ChannelHandler`, remove retired channels, merge `dev`, or push. Those
actions remain an atomic C6 responsibility.
