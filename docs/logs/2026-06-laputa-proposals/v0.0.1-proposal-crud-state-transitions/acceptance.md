# Story 1.3 Acceptance

Acceptance checks from user/product perspective:

- Create a valid `EvolutionProposal` with evidence refs through `ProposalRepository::create_proposal`.
- Fetch it with `get_proposal` and list it through `list_summaries`.
- Confirm summary fields include status, proposal type, target section, source run id, risk, created/updated timestamps, and evidence count.
- Edit a pending proposal and confirm evidence refs are preserved unless explicitly replaced.
- Move proposals through valid transitions such as `pending_review -> approved -> applied -> reverted`.
- Attempt an invalid terminal-state transition and confirm a typed error is returned while persisted state remains unchanged.
