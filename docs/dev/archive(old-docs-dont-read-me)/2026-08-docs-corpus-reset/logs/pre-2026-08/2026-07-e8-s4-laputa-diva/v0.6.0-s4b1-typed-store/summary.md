# S4b-1 Typed Laputa Store

> Repatriated evidence from `refactor/deep-governance` on 2026-07-30.
> This implementation is a port source, not proof that current
> `agent-diva-pro` already uses the typed store.

Laputa persistence now uses a profile-local typed `laputa.sqlite3` adapter owned by `agent-diva-state`. The domain depends on `LaputaRecordStore`, not `StateStore`, and no runtime registry/blob reader remains.

The schema includes typed memory and proposal rows, FTS5, version metadata, persona/L1 projection tables, and dormant sync journal tables. Existing proposal, decision, migration, restart, and profile-isolation behavior remains operational.
