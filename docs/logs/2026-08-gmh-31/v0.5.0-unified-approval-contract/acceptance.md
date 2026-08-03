# Acceptance

## Automated story acceptance

1. Create Command, Plan, and Memory pending approvals and confirm the unified list/detail projection reports their durable state and safe presentation.
2. Submit decisions with `expected_version` and `idempotency_key`; confirm stale clients receive typed 409 responses and exact retries do not repeat effects.
3. Reconnect the SSE endpoint after its durable cursor; confirm ordered `approval.requested`, `approval.resolved`, and `approval.updated` facts without domain mutation on disconnect.
4. Parse the shared Rust fixture through the TypeScript guard and build the GUI/Tauri bridge without changing legacy payloads.
5. Confirm malformed JSON/query input and invalid cursors fail with typed 422 responses.

These conditions are covered by the tests listed in `verification.md`.

## Deferred human acceptance

No intermediate human action is required. Badge/drawer behavior, keyboard flow, reconnect presentation, debug/release desktop startup, and the real once-only execution observation belong to the single final Epic/M3 manual smoke after GMH-32 and GMH-33 are complete.
