# Summary

- Fixed the CLI TUI logging bug where tracing output was written directly to the active terminal and corrupted the ratatui display.
- Added a logging initialization path that keeps file logging enabled while disabling terminal log output for TUI commands only.
- Extended the same clean-output treatment to `agent --message` so one-shot agent calls no longer print startup tracing noise to the terminal by default.
- Kept existing terminal logging behavior unchanged for interactive chat and other non-TUI CLI commands.
