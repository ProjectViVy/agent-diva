# Ask Mode Read-only Boundary Release

This is a backward-compatible security fix. The desktop request shape remains
`mode: "ask"`; Agent and Plan behavior remain unchanged.

No push or deployment is performed. A real desktop smoke must confirm that an
Ask request cannot execute a command or delete a disposable file.
