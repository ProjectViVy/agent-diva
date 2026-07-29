# Release

No deployment or push was performed. The delivery is split into focused local commits for execution-context behavior, the DeepSeek fixture, and iteration evidence.

SQLite migration is idempotent through `CREATE TABLE IF NOT EXISTS`. Existing plans remain untouched; an approved execution without a context row is handled as a legacy state and requires a uniquely resolvable continuation.
