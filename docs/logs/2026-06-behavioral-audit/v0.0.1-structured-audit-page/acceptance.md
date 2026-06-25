# Acceptance

1. Start the gateway or bundled GUI so runtime logging is active.
2. Trigger at least one agent turn that causes a decision point and a tool call.
3. Open GUI Settings -> Audit.
4. Select the current date and confirm the structured event tab lists audit events with timestamps and JSON payload details.
5. Switch to the raw log tab and confirm the page shows the underlying `gateway.log.YYYY-MM-DD` contents for the selected date.
6. Change the date picker to a day without logs and confirm the page shows an empty-state message instead of failing.
