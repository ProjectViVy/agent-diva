# Release

## Delivery Method

No separate deployment workflow was required for this repository-local change.

## Runtime Effect

- Structured runtime JSONL logging is enabled by default.
- Runtime logs are retained for 7 days by default.
- Runtime logs write to `logging.runtime_log_dir` when configured, otherwise
  reuse `logging.dir`.

## Deferred Scope

- No manager debug bundle export yet.
- No gateway/channel end-to-end trace wiring yet.
- No GUI observability settings page yet.
