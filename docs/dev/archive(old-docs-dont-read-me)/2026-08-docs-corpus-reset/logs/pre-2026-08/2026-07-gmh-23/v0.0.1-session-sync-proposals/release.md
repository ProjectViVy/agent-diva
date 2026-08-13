# GMH-23 Stage 1 Release

This stage is committed locally as `15ad2f8f` (`feat(memory): proposalize Laputa
turn sync`). It is not pushed or deployed.

The change is active only when the selected memory provider is
`LaputaMemoryProvider`. Legacy Markdown persistence remains unchanged. Rollback
is the focused revert of `15ad2f8f`; pending proposals created while the change
was active remain reviewable data and must not be silently deleted.
