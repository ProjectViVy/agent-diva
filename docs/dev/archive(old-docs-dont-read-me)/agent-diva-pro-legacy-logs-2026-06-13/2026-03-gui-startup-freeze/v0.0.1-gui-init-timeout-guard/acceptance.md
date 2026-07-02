# Acceptance

1. Launch `AGENT-DIVA-GUI` when the local gateway is healthy and confirm the splash screen closes normally.
2. Simulate a stalled local API or partially hung gateway process and confirm the GUI still enters the main page instead of staying on the initialization splash forever.
3. Confirm session recovery is still attempted, but a stuck session-history request no longer blocks GUI entry.
4. Check the console for warning logs that identify which startup step timed out, to support future root-cause tracing.
