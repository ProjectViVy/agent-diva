# Acceptance

## Product Checks

- From Chat, trigger AutoDream manually and verify the pending run card updates after polling.
- When an AutoDream run completes with proposal IDs, verify compact proposal cards appear in the chat timeline.
- Open Evolution from a proposal/run link and verify the source-run filter only scopes that deep-linked view.
- Navigate to Evolution normally and verify stale source-run/proposal deep-link state is cleared.
- Defer a proposal and verify the backend transition uses state `deferred`.
- Batch reject multiple proposals and verify a confirmation is shown before backend calls are made.
- Force one batch transition to fail and verify partial failure feedback is shown.
- Open Policy and verify the settings button navigates to Settings > Self Evolution.

## Technical Checks

- Confirm serialized `ProposalState::Deferred` uses `deferred`.
- Confirm Laputa permits `pending_review/edited/needs_attention -> deferred` and `deferred -> pending_review/rejected/edited/superseded`.
- Confirm migrated configs include default self-evolution settings.
