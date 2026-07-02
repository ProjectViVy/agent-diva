# Story 1.1 Acceptance

## Acceptance Steps

- Confirm `agent-diva-core` exposes `agent_diva_core::evolution`.
- Confirm exported types include `EvidenceRef`, `EvidenceSource`, `EvolutionProposal`, `ProposalType`, `ProposalState`, `LaputaSectionName`, `ChangelogRecord`, `AuditEvent`, `RollbackRequest`, and `AutoDreamRunRecord`.
- Confirm JSON uses snake_case variant strings such as `memory_patch`, `memory_md`, and `pending_review`.
- Confirm all eight v1 proposal types route to canonical Laputa sections.
- Confirm unknown raw proposal type strings return `EvolutionError::UnknownProposalType`.

## Result

Accepted by automated tests for serde round trip, route mapping, and unknown-type error behavior.
