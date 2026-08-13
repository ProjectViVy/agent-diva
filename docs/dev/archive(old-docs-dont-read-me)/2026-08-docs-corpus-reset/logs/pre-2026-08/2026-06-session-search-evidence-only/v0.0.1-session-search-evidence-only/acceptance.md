# v0.0.1 Session Search Evidence Only Acceptance

1. Search session history from the Notebook/report proposal flow and confirm each hit returns `session_id`, timestamp, bounded snippet, source URI, and session evidence metadata.
2. Confirm malformed session JSONL files are skipped while diagnostics are still returned.
3. Attach selected session hits to a Notebook proposal preview or create request and confirm the emitted proposal uses `EvidenceSource::Session` refs alongside report evidence.
4. Confirm the default runtime prompt/context does not automatically include raw session search hits unless a later approved Laputa apply path makes them authoritative.
