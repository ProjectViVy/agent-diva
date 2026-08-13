# Story 6.2 Summary: Recovery, Conflict, and Typed Error Handling

## Changed

- Added migration file recovery snapshots so section and state writes can restore the prior safe state when commit fails.
- Kept apply recovery on the existing proposal lock path and exposed a service-level apply-with-options path for failure-injection validation.
- Marked unresolved apply conflicts as `needs_attention`.
- Added canonical `LaputaError::code()` values, including `schema_incompatible`, `unknown_layer`, `conflict_unresolved`, `rollback_expired`, `lock_timeout`, and `io_error`.
- Updated manager Laputa HTTP error serialization to preserve concrete Laputa error codes.
- Emitted Laputa error diagnostic events for apply failures through existing event plumbing.

## Impact

- Authority writes are more recoverable after apply and migration failures.
- UI and HTTP callers receive stable typed error codes instead of a generic `laputa_error`.
- Conflict handling now leaves proposal state explicit for operator review.
