# Acceptance

1. Start DIVA and expand the left navigation.
2. Confirm Chat, Pet, Console, Cron, Tools, MCP, Skill, and Settings share aligned left and right edges; nested items may keep their internal text inset.
3. Confirm the navigation rows have consistent 40px hit areas and active, hover, focus, and inactive colors follow the selected theme.
4. Switch among Love, Dark, Default, and Miku and verify that selected rows remain legible with the correct icon contrast.
5. Open session, Cron, MCP, and provider deletion confirmations; verify the dialog shows pending state until the operation completes and keeps an actionable error visible when it fails.

The browser smoke verified layout in Love theme. Theme-state consistency is covered by tests for all four themes. Backend-dependent dialog flows require a running local service for manual acceptance.
