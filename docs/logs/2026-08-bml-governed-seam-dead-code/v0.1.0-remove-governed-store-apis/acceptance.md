# Acceptance

1. Memory CRUD still goes through `TypedMemoryStore::put` / `put_tombstone`
   / `import_records` with no approval receipt.
2. `put_governed` and `rollback_governed` are not callable from production
   code; `just bml-boundary-check` fails if a governance module reintroduces
   `.put_governed(` or `.rollback_governed(`.
3. Opening an existing `{config_dir}/memory/memory.sqlite3` still succeeds
   without a schema bump (`SCHEMA_VERSION` remains 1; journal table still
   created with `IF NOT EXISTS`).
4. No Manager/GUI/AutoDream Memory write contract change.
