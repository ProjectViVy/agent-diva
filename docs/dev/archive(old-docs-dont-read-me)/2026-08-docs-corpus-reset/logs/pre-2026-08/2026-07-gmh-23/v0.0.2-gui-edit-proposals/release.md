# GMH-23 Stage 2 Release

Local focused commits:

- `27789fe7` — Laputa/Manager user-edit proposal boundary.
- `ded455fa` — Persona/Memory GUI proposal submission behavior.
- `932251b5` — complete SectionEditor regression alignment.

No push or deployment was performed.

The existing `/api/laputa/section/:name/write` route and
`laputa_write_section` command remain available, but now return a pending
proposal result. Consumers must treat `changelog_id` and `applied_at` as
optional and must not infer that authority changed.

Rollback is the focused revert of the commits above. Pending proposals already
created are governance records and must not be silently removed.
