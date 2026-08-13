# v0.0.1 Session Search Evidence Only Summary

- Added a read-only session JSONL search service in `agent-diva-core` with bounded snippets, per-file diagnostics, and stable session evidence URIs.
- Added conversion from session search hits into `EvidenceRef { source: Session, uri, excerpt, hash, created_at }`.
- Exposed Notebook/Tauri session evidence search plus optional attachment of selected session hits into proposal preview and creation flows without routing through `MemoryProvider` or direct authority writes.
- Added regression coverage to prove session search results remain evidence-only and do not appear in default runtime prompt context.
