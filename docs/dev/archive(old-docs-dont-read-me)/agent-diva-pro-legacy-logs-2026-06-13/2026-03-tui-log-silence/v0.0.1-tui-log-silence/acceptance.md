# Acceptance

1. Run `agent-diva tui`.
2. Enter a prompt that triggers normal agent processing.
3. Confirm the TUI timeline continues to render cleanly without raw tracing lines being inserted into the terminal.
4. Exit TUI and confirm log files are still written under the configured logging directory.
5. Run a non-TUI command such as `agent-diva status` and confirm normal terminal logging behavior remains available.
