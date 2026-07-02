# Epic 2/3 Review Remediation Summary

## Scope

Implemented the approved remediation plan from the Epic 2/3 combined review.

## Changes

- Added durable `deferred` proposal state across core contracts, Laputa transitions, GUI types, and Evolution actions.
- Added self-evolution config schema plus manager HTTP/Tauri/desktop API surfaces for reading and saving policy settings.
- Updated Chat AutoDream trigger flow to poll run status and append generated proposal cards when available.
- Hardened Evolution navigation, deep-link handling, source-run filtering, batch reject confirmation, batch partial-failure reporting, and null/array guards.
- Added Evolution i18n strings and compact source-run filter styling.
- Fixed a related config migration initializer so migrated configs include default self-evolution policy.
- Fixed the manager AutoDream error response match for `InputCollection` and `ProposalPersistence`.

## Impact

The Evolution UI now uses backend-durable defer transitions instead of local-only markers for the primary defer action. Self-evolution settings can be managed through the same config path used by manager and Tauri. Chat-triggered AutoDream runs now surface generated proposals back into the chat workflow.
