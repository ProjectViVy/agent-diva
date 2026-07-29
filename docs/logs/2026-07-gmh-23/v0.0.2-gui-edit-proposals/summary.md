# GMH-23 Stage 2 Summary

## Outcome

Persona/Memory section edits now create governed pending proposals instead of
immediately applying authority.

- The existing Manager route and Tauri command names are retained.
- Laputa validates the target and patch, assigns a proposal type and fail-closed
  risk, records bounded `UserInput` evidence, and stops at `pending_review`.
- The response returns proposal identity, type, risk, and state; changelog and
  apply timestamps remain absent until an explicit apply.
- The GUI uses proposal language, restores the applied authority after
  submission, shows a pending-review toast, and refreshes the Evolution badge.
- GUI proposal types now include history, daily, weekly, and monthly patches.

Laputa migration still uses its existing atomic bootstrap path. Its write
cutover remains gated by GMH-24 shadow comparison and rollback validation.

GMH-23 remains open for policy-controlled low-risk auto-apply and high-risk
HITL/edit/conflict/compensation/forgetting flows.
