# Remove dead ProposalCreated memory outcomes

- Date: 2026-08-23
- Slice: `MEMORY-CRUD-PROPOSAL-CREATED-DEAD-ENUM`
- Branch / worktree: `chore/memory-crud-proposal-created-dead-enum` /
  `C:\Users\Administrator\Desktop\morediva\agent-diva-memory-crud-enum`
- Base: local `dev` @ `41e8e23e`

## Goal

Delete the public `ProposalCreated` outcomes left after S5 made Memory CRUD
direct. Production `MemoryHome` no longer produced them.

## Changes

- Removed `MemoryCrudOutcome::ProposalCreated` and its serde roundtrip case.
- Removed `SyncTurnStatus::ProposalCreated` and updated `sync_turn` rustdoc.
- Consolidation no longer counts or treats proposal-created as success;
  fallback `sync_turn` accepts only `Persisted` / `Noop`.
- Deleted `ProposalCreatingProvider` and
  `sync_turn_proposal_created_is_treated_as_success`.
- Prompt negative assertion (`proposal_created` must not appear) kept.

## Out of scope

- BML `put_governed` / schema (separate worktree).
- Skill `memory_distill` `request_created`.
- Historical research snapshots.
