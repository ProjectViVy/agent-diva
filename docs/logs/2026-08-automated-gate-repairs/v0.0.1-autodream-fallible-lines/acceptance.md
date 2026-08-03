# Acceptance

1. A valid run event remains visible through `list_run_events`.
2. A malformed JSON line is skipped without exposing payload data.
3. Invalid UTF-8 or another line-read failure returns a contextual I/O error.
4. The AutoDream Clippy target no longer reports `lines_filter_map_ok`.
