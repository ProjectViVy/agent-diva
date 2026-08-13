# Acceptance

1. In the GUI chat view, send a message with `plan` mode.
2. After the plan writes todos or reaches `AwaitingApproval`, verify the current conversation shows the inline plan card instead of requiring a navigation jump to `PlanningView`.
3. When the plan reaches approval, verify the chat window shows the inline approval bar with `Approve and Execute` and `Modify Plan`.
4. Click `Approve and Execute` and verify the same conversation advances into execution mode and shows the inline progress strip.
5. Open the legacy planning page from navigation and verify the plan list/detail page still loads as a secondary surface.
