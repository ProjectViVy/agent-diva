# GMH-23A Summary

Frozen Embedded Laputa as Diva's only local Memory store/retrieval layer.
The current branch uses `.laputa/memory.sqlite3`, canonical Core
`MemoryRecord`, workspace `sqlx`, transactional revision CAS, tombstones, and
FTS5 candidates. Existing file-first authority remains unchanged.
