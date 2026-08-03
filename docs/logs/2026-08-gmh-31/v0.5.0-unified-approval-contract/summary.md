# GMH-31 Unified Approval Contract

## Outcome

- Added one Manager approval service that projects Command, Plan, and Memory governance from the shared durable coordinator without becoming a second authority.
- Added stable list, detail, decision, cancel, and durable cursor SSE routes under `/api/approvals`.
- Added typed HTTP reason codes, including malformed body/query handling, while preserving the existing Command, Plan report, Laputa, and legacy SSE contracts.
- Added Tauri list/detail/decide/cancel/stream bridges as transport-only adapters and a TypeScript runtime guard backed by the Rust fixture.
- Added append-ordered ledger event pagination and corrected explicit Sandbox timeout materialization at the exact durable expiry boundary.

## Safety and compatibility

The ledger stores canonical digests and governance metadata only. Command text, cwd, approval reason, Plan markdown, and Memory proposal payload remain in their owning process/domain stores. No raw command replay or restart waiter recovery was added.

Plan editing remains on the revision API and Memory editing remains on the proposal API. The unified API exposes `edit`/`apply` intents but does not accept arbitrary domain payloads.

No provider, key, real profile, network service, administrator privilege, system security policy, push, or manual desktop run was used.
