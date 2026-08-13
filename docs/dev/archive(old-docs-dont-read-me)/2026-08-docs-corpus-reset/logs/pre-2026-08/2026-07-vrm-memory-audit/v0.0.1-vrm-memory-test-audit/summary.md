# Summary

## Scope

This iteration re-audits `origin/vrm-memory-test` against the current `agent-diva-pro` mainline only for:

- `agent-diva-core/src/memory/*`
- `agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva-agent/src/tool_assembly.rs`
- `docs/dev/archive/memory-evolution/*`

The objective is to recover still-valuable memory or Mentle behavior without regressing the current architecture.

## Findings

### 1. The branch does not contain an `agent-diva-memory` crate

`git ls-tree -d --name-only origin/vrm-memory-test` shows no `agent-diva-memory/` directory and no extra workspace member. The earlier "recover the missing crate" framing was incorrect for this source branch.

### 2. `loop_turn.rs` and `tool_assembly.rs` are already superseded on `agent-diva-pro`

The branch version predates the current mainline additions around:

- plan mode and planning tools
- read-only mask enforcement
- security filtering and audit emission
- token ledger and session budget checks
- context compaction
- background task registration
- global tool timeout wiring

Reintroducing the old branch files would remove or weaken current behavior. These differences are classified as `already superseded` or `unsafe to reintroduce`.

### 3. The only valuable code delta in `MemoryManager` has already been absorbed

Compared to `origin/vrm-memory-test`, the valuable delta in `agent-diva-core/src/memory/manager.rs` is the use of `tokio::task::spawn_blocking(...)` inside `sync_turn()` for `MEMORY.md` and `HISTORY.md` persistence. The current mainline already contains that logic, so no further code migration was required in this pass.

### 4. The memory-evolution docs are branch-era design intent, not missing runtime behavior

The removed `docs/dev/archive/memory-evolution/*` files describe:

- diary domain positioning
- future recall slots
- retrieval-ready metadata
- diary tool contract ideas
- a hypothetical future `agent-diva-memory` crate

These are largely design documents. They were not a proof that the branch shipped a separate runtime crate.

### 5. The design intent is still partially useful, but only as follow-up work

The still-useful parts are:

- keep diary as one memory domain, not "the whole memory system"
- define future recall contracts explicitly instead of bolting them into prompt assembly
- keep any diary tooling behind the current `MemoryProvider` / runtime seams

Those ideas should be re-specified on top of the current mainline architecture, not restored verbatim from the old branch.

## Diff Classification

| Area | Classification | Outcome |
| --- | --- | --- |
| `agent-diva-core/src/memory/manager.rs` async persistence | `still valuable`, already landed on mainline | No code change needed |
| `agent-diva-agent/src/agent_loop/loop_turn.rs` older prefetch/save-turn behavior | `already superseded` | Do not reintroduce |
| `agent-diva-agent/src/tool_assembly.rs` older tool wiring | `unsafe to reintroduce` | Do not reintroduce |
| `docs/dev/archive/memory-evolution/*` branch docs | `doc-only / never implemented` | Preserve only as historical design input |
| Hypothetical `agent-diva-memory` crate | `not present in source branch` | Explicitly not part of this migration |

## Changes Made

- Recorded this audit in `docs/logs/2026-07-vrm-memory-audit/v0.0.1-vrm-memory-test-audit/`
- Added a deferred backlog item to write a current-baseline memory interfaces spec
- Made no runtime code changes beyond the audit record, because the branch's only worthwhile code delta is already present on `agent-diva-pro`

## Impact

This pass closes the mistaken recovery path that assumed `vrm-memory-test` contained a standalone memory crate. It protects the current Laputa/Mentle/memory-boundary architecture from accidental rollback and narrows future memory work to explicit, separately scoped stories.
