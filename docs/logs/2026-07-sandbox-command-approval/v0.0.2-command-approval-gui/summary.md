# Sandbox command approval GUI closure

The desktop GUI now consumes the Manager command-approval contracts end to end. Tauri proxies pending queries and decisions, bridges the dedicated SSE stream, reconnects automatically, and notifies the frontend to reconcile missed requests after every connection.

The GUI keeps a deduplicated, creation-ordered queue across GUI sessions. Requests show command, working directory, reason, source session, and the server-owned deadline. A request can be rejected, approved once, or approved for the exact source session only after that session is selected.

Persistent global safe-prefix rules remain intentionally deferred to Phase 3.
