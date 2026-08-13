# Story 3.4 Acceptance

1. Trigger an AutoDream run and complete reflection output assembly.
2. Confirm `.agent-diva/autodream/runs/{run_id}/autodream_run.json` exists and includes:
   - `schema_version`
   - bounded `evidence_refs`
   - `confidence`
   - `output_summary`
   - `proposal_candidates`
   - `proposal_ids`
   - `review_required: true`
3. Confirm corresponding proposals exist in Laputa with:
   - shared `EvolutionProposal` fields
   - `state = pending_review`
   - `source_run_id = {run_id}`
4. Confirm `.agent-diva/autodream/events.jsonl` received proposal/output events.
5. Confirm the run `record.json` summary and `proposal_ids` reference the emitted proposals.
