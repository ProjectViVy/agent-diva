# Current state and receiving-agent handoff

## Goal

Deliver six native external-platform adapters—Telegram, Discord, Feishu, DingTalk, Email, and
QQ—on the Super Channel Fabric. Use Octos as a pinned capability and test reference while keeping
Agent Diva's stronger transport, governance, bounded runtime, typed envelope, and Rust 1.80
constraints.

## Verified state at handoff

- Root coordination tree: `dev`; inspected at `84a5172f` before the C5-P coordination-lock commit.
- Implementation tree: clean `feat/channel-epic` at `90eac026` before C5-P documentation.
- C1 contracts, C2 Fabric/runtime, C3 Neuro-Link gateway/journal, and C4 desktop migration are
  implemented on the isolated branch.
- The six active files still implement only legacy `ChannelHandler`; `rg "impl ChannelAdapter"`
  finds no real-platform implementation.
- `AdapterRegistry`, `AdapterPacingLane`, `AdapterSupervisor`, capability checks, retry safety,
  partial-delivery receipts, and Fabric admission already exist. Do not replace them with Octos
  `ChannelManager`, bus, or coalescing.
- Manager does not yet assemble the native adapter registry. That cutover is intentionally C6.
- C4 automated workspace/GUI gates passed. Real Tauri disconnect/replay acceptance remains open
  and must pass before C6.
- User decision: QQ is the required live C5 vertical smoke.

## First safe actions

1. Inspect root and isolated `LOCK.md`; stop on an overlapping live claim.
2. Confirm both trees with `git status --short --branch` and never develop in the root tree.
3. Confirm the Octos pin with
   `git -C C:\Users\Administrator\Desktop\morediva\.workspace\octos rev-parse HEAD`.
4. Read the documents in the order listed by `README.md`.
5. The Lead freezes shared APIs and commits them before creating platform worker branches.

## Owned scope

- `agent-diva-channels/src/adapters/**`
- narrowly required shared adapter/runtime interfaces and tests
- `agent-diva-channels/tests/fixtures/c5/**`
- this C5 documentation package, iteration log, and C5 TODOLIST entries
- Manager attachment-service implementation and production registry assembly only when C6 starts

## Do not touch in C5

- GUI/Neuro-Link protocol behavior completed by C3/C4
- provider routing or provider model-ID behavior
- Laputa/BML, Sandbox, Approval, Planning, or Agent admission semantics
- legacy deletion list, retired channel source, or production Manager cutover
- root `dev` history except lock coordination; no merge or push

## Stop conditions

- Octos SHA differs from the pinned value.
- A mandatory capability cannot be backed by a platform operation and fixture design.
- A worker needs to alter `ChannelAdapter`, `ContentPart`, `ChannelCommand`, Fabric capacity, or
  shared config after the shared-contract freeze.
- A proposed dependency raises MSRV, introduces Rust 2024 requirements, or duplicates existing
  Fabric/pacing/supervisor behavior.
- Live credentials would need to be stored or printed.

In any stop condition, record a concise request in `decision-requests.md`, notify the Lead, and do
not continue by inventing a compatibility shim.
