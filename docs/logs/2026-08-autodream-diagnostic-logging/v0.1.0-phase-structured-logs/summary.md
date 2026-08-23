# Summary

Close `AUTODREAM-DIAGNOSTIC-LOGGING` by adding phase-level structured logs
on the S3 AutoDream worker path.

## 改了什么

- New `agent-diva-autodream/src/diagnostics.rs` emits the same payload to
  `tracing` and durable `events.jsonl`: `run_id`, `phase`, input summary,
  gate rejection, `proposal_id`, failure code.
- `AutoDreamWorker` logs orient/gather/consolidate/propose, input
  collection, ACTMEM Work retry, Skill request create/skip/reject, and
  terminal success/failure.
- `AutoDreamEvent` gained optional structured fields; HTTP event list
  serializes them when present.
- Tests pin the contract in `tests/diagnostics.rs` and extend
  `tests/current_contract.rs` plus existing S3 worker unit tests.

## 影响范围

- Production: `agent-diva-autodream` worker/service event schema only.
- No MemoryPatch / BML write / STM-as-proposal / REDLINE / DREAM / user
  preference changes.
- Manager HTTP `/autodream/runs/:id/events` returns extra optional JSON
  fields; old events still deserialize.

## 不做

- Did not restore EvolutionProposal / SopCreate / Governance apply.
- Did not convert STM into proposals.
- Did not add GUI event rendering.
