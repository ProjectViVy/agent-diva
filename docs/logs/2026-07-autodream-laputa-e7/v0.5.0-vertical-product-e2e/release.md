# Release

Typed authority treats canonical `MemoryRecord.content` as content, rather than
requiring it to be a legacy section JSON projection. Legacy projection writes
retain their existing JSON validation.

Canonical writable opens require the database to exist when requested and
retain identity/integrity checks; authority operations no longer receive a
read-only SQLite connection.

No push or deployment was performed. Final E7 workspace, GUI, clean-break, and
recovery gates remain pending.
