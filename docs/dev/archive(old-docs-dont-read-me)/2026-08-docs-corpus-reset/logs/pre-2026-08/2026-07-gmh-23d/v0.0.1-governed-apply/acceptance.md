# GMH-23D Governed Apply Acceptance

Automated acceptance confirms that direct approved/rejected transitions are
blocked, governance decisions create bound receipts, edits invalidate old
requests, and the typed executor has no production registration.

Required real desktop acceptance:

1. Approve-only a medium-risk proposal and record request/proposal ID.
2. Reject another proposal and confirm it cannot be applied.
3. Edit a proposal, approve it, and confirm the pre-edit request is stale.
4. Double-click approve-and-apply from two windows and check for one outcome.
5. Restart after authorization and verify the proposal/receipt state recovers.
6. Apply and roll back a proposal, retaining request/proposal/audit IDs.

Any duplicate execution or unrecoverable state blocks G2D and must be recorded
with those IDs. G2D is not marked complete before this desktop run.
