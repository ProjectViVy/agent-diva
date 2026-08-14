# I1-S3 Memory / ACTMEM / Recap — Acceptance

## Automated acceptance

1. Confirm a read-only first run does not create `memory.sqlite3`, ACTMEM, or MEMRULES files.
2. Create, edit with the current revision, conflict with a stale revision, and soft-delete a BML record; confirm no Approval Center entry is created.
3. Write and read ACTMEM sections, verify caps/CAS/no-op behavior, fold one idle session, and inspect/delete only server-projected capsules.
4. Run an interactive turn and observe immediate Pulse then Recap; verify cron and subagent turns do not write them.
5. Activate a deferred Memory/ACTMEM write tool and confirm the next model call sees full MEMRULES first.
6. Run AutoDream and confirm Work is organized without MemoryPatch, BML writes, or S3 proposals.

## Native desktop acceptance still required

1. Launch Manager and the Tauri desktop app against an isolated config directory.
2. Complete BML create/edit/soft-delete and confirm draft/selection survive a rejected CAS save.
3. Edit Pulse, Recap, and Work explicitly; inspect and delete one capsule with a single confirmation.
4. Observe MEMRULES default source, save a user file, and observe the source switch.
5. Confirm narrow-screen tabs use the same BML / ACTMEM / MEMRULES tasks and no Memory action opens or populates Approval Center.

Record the native observations in this file and then close `MEMORY-S3-DESKTOP-SMOKE` and `UI-S3-MEMORY-ACTMEM` in `TODOLIST.md`.
