# Acceptance

1. Generate a plan in Plan mode and confirm the turn stops at the approval state.
2. Approve the plan from the GUI and confirm the runtime phase becomes `Execute`.
3. Let execution finish; the agent must pass through `Verify` before `Completed`.
4. Confirm no `Invalid transition: Plan → Completed` error is shown.

