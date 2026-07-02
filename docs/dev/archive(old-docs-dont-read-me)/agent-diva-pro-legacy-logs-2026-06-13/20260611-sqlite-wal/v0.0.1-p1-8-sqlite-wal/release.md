# P1-8 SQLite WAL Release

## Method

No special release step is required. The change is included in the Rust workspace build and takes effect when `SqlitePlanningStore::new` initializes a planning database.

## Rollout

- Deploy through the normal application release process.
- Existing planning databases do not need schema migration; PRAGMA settings are applied at initialization time.

