# Summary

- Fixed the planning approval tool so approval is an atomic transition from `AwaitingApproval` to `Execute`.
- Removed `plan_approve` from Plan mode's allowed tool set, preventing the planning turn from bypassing the approval boundary.
- This prevents a plan left in `Plan` from being incorrectly completed directly by a model tool call.

