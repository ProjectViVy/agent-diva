# Ask Mode Read-only Boundary Summary

Fixed a production authority bypass where the desktop sent `exec_mode=ask`
but AgentLoop classified only Plan mode and therefore ran Ask turns with the
normal Agent tool surface.

Ask is now a stable turn mode. It does not inherit approved execution state,
receives an explicit read-only prompt, exposes only the five established
inspection tools, and is enforced again immediately before tool execution.
Unknown explicit modes fail closed to Ask.
