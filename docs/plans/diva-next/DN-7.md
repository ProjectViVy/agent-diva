# DN-7 — Historical import cancelled

Owner decision, 2026-10-03: do not import old data because there are no current
DIVA users to migrate. Status/dependencies: [index.md](index.md).

This supersedes the DN-P1 historical-import requirement and old DN-7 plan.
No importer, dry-run receipt workflow, compatibility bridge or migration
fixtures are required. DN-8 has no dependency on DN-7.

Use fresh DIVA Next state. Do not automatically locate/read/copy the old home,
dual-write, delete old databases, or present cancelled migration as implemented.
New voice/reference/VRM asset import remains a separate scoped native feature.
The old plan remains recoverable in git history.
