# GMH-24A Acceptance

1. Run `agent-diva-migrate memory dry-run` with an explicit source root,
   source file, workspace, and workspace ID.
2. Confirm the JSON report contains a stable migration ID and digests while
   the target workspace remains unchanged.
3. Run the same arguments with `memory apply`; confirm the manifest, backup,
   store revision, and clean integrity summary.
4. Repeat apply and confirm no duplicate record is created.
5. Run `memory rollback --migration-id ...`; confirm the pre-import record
   count and integrity are restored.
6. Confirm a source under `.mentle`, a symlink, an escaping path, or an unknown
   file type is rejected before any import write.
