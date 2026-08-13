# Verification

- Static trace confirmed the GUI defaults approval to `Compact`.
- Static trace confirmed execution-start `Compact` retains only six local history messages and does not persist a summary or boundary.
- Existing generic compaction E2E coverage validates auto-compaction ordering, but no test covers the Plan Mode policy path.

No build or test command was run for this documentation-only update.
