# Story 1.1 Governance Domain Types Summary

## What Changed

- Added `agent-diva-core/src/evolution/` and exposed it from `agent-diva-core/src/lib.rs`.
- Defined shared governance contracts for evidence, proposals, proposal state, Laputa section names, changelog records, audit events, rollback requests, and AutoDream run records.
- Implemented v1 proposal type routing to canonical Laputa sections with typed unknown-type errors.
- Added focused unit tests for serde stability, route coverage, and unknown proposal type handling.

## Impact Range

- Affects `agent-diva-core` public domain types only.
- Does not add Laputa storage, manager routes, Tauri commands, provider integrations, or GUI DTOs.
