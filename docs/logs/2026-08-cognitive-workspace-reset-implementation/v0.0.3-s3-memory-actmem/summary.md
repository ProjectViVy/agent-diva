# I1-S3 Memory / ACTMEM / Recap — Summary

## Outcome

Implemented the config-rooted, machine-wide Memory workspace described by D2 and D4 S3.

- Added shared `MemoryHome` under the BML logical layer. Production BML now uses `{config_dir}/memory/memory.sqlite3`, opens lazily, never imports the workspace legacy database, accepts LongTerm only, and exposes revisioned direct CRUD with atomic tombstones.
- Added strict ACTMEM Markdown storage at `{config_dir}/actmem/ACTMEM.MD`, bounded Pulse/Recap/Work, CAS/no-op behavior, per-session idle folding into safe 800-character capsules, and server-projected capsule access.
- Renamed the runtime concept to `SessionCheckpoint`, moved its serialized name to `session_checkpoint`, and connected explicit reset/delete/end cleanup while preserving checkpoints across daemon shutdown.
- Added per-turn Pulse and immediate mechanical Recap writes, cancellable ten-minute idle generations, and cron/system/subagent exclusions.
- Added CORE read tools, DEFER write tools, write-time full MEMRULES injection, and no Memory/ACTMEM tools for subagents.
- Added AutoDream Work organization with one Pulse/Recap-only conflict retry and zero S3 MemoryPatch, BML writes, or Memory approvals.
- Added Manager HTTP routes, Tauri mirrors, and a responsive BML / ACTMEM / MEMRULES desktop workspace without `open-approval` flow.

## Remaining boundary

S4 owns Evolution/Skill proposal creation. S5 owns physical deletion of retired symbols and old endpoints; S6 owns final zero-residual proof. Real native desktop interaction acceptance remains open in `TODOLIST.md` even though automated GUI and Tauri gates pass.
