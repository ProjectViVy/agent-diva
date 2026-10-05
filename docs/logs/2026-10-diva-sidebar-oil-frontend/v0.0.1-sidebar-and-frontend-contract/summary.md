# DIVA sidebar and frontend contract cleanup

## Scope

Unified the expanded navigation sidebar's row sizing, horizontal alignment, and theme-semantic colors. The main navigation, tool group, nested links, and settings row now share a 40px row height and align to the same usable width after the stable scrollbar gutter. The menu control and settings footer use that same width. Active, hover, and inactive states remain distinct while consuming the same theme tokens.

The wider Oil Frontend pass corrected frontend contracts found during review:

- Removed session pin controls that emitted a no-op event; recorded persistence as deferred until a write API exists.
- Removed disabled chat toolbar placeholders for Edit, Rewind, and Fork.
- Kept async destructive confirmations open while requests run and exposed retryable errors; migrated session, Cron, MCP, and custom-provider deletion flows.
- Separated Cron loading failures from genuine empty results, retained the current plan if a new session fails to load, and made the tools navigation group a semantic button.
- Consolidated shared chat UI types and default preferences.

Vue props and business API behavior remain unchanged apart from removing the unused pin UI event and stale fields. No backend contract or desktop packaging code changed.

## Impact

The changes affect DIVA GUI navigation, chat actions, destructive dialogs, Cron and provider settings, shared UI types, translations, and related tests. Sidebar layout, route order, responsive behavior, and theme identities are preserved.
