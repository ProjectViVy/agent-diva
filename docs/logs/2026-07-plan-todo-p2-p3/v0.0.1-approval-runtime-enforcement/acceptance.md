# Acceptance

1. Create a plan and move it to `Plan`.
2. Submit a complete plan with scope and verification metadata; confirm it enters `AwaitingApproval` with a revision.
3. Attempt a filesystem, shell, TODO-write, network, MCP, spawn, or scheduling tool before approval; confirm runtime policy denies it.
4. Approve with the displayed revision and optional TODO choice; confirm the receipt/event and execution projection are consistent.
5. Edit submitted plan content; confirm the plan reopens to `Plan`, its revision increases, and the old approval request fails.
6. For a materialized TODO list, attempt a whole-list replacement; confirm it is rejected without deleting the existing items.
