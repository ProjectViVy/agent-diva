# Acceptance

1. Rebuild and restart the desktop application from this commit.
2. Start one AutoDream run from Evolution and open its live monitor.
3. Confirm normal responses show a compact JSON object containing `schema_version`, `candidates`, and `diagnostic_codes` only.
4. If the provider emits invalid JSON, confirm the monitor shows `[JSON format retry 2/3]` and, if needed, `[JSON format retry 3/3]`.
5. Confirm a repaired response proceeds through Candidate Gate normally.
6. Confirm three invalid responses end with `reflection provider returned an invalid schema` and create no proposal or Memory change.
