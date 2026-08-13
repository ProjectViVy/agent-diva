# GMH-23 Stage 2 Acceptance

## Real desktop smoke required

Environment:

- Windows desktop capable of starting the Tauri main window.
- Embedded Manager connected to a disposable Laputa workspace.
- An initialized identity section whose exact authority content is known.

Steps and observations:

1. Open **Persona & Memory**, select identity, edit its content, and click
   **Submit proposal**.
2. Confirm the dialog describes governance review rather than overwrite/apply.
3. Confirm the success toast says the proposal is pending review.
4. Confirm the editor returns to the previously applied identity authority.
5. Confirm the Evolution badge refreshes and its inbox contains a high-risk
   `identity_patch` in `pending_review`.
6. Confirm the identity section and changelog remain unchanged before approval.
7. Force a Manager failure and confirm the draft remains editable with an error.

If any step fails, retain:

- Manager and Tauri logs around the request.
- Proposal ID and the `/write` response body.
- `.laputa/events.jsonl` entries for the proposal.
- Before/after identity section and changelog directory listings.
- A screenshot of the editor, toast, badge, and proposal detail.

This manual smoke is pending. Automated tests and builds do not replace it.
