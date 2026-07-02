# Story 1.4 Summary

## What Changed

- Added `ProposalRepository::apply_proposal` as the Laputa-owned authority mutation entrypoint.
- Added apply-time validation for approved proposal state, proposal type routing, writable target sections, JSON schema compatibility, and unresolved conflicts.
- Added rollback staging before section mutation, changelog/audit writes as part of apply, and final proposal transition to `applied`.
- Added deprecation apply behavior that records changelog/audit without mutating arbitrary authority section content.
- Added deterministic failure injection for rollback tests.

## Impact Range

- Backend-only change in `agent-diva-laputa`.
- No manager, Tauri, GUI, channel, provider, or CLI surface added.
- Existing proposal CRUD and storage APIs remain compatible.
