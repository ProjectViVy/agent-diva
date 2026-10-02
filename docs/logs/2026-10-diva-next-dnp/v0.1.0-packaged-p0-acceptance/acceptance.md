# DN-P acceptance

## Gate decisions

- Task 1 artifact verification: **PASS** — deb runtime bytes identical to DN-L pins.
- Task 2 deterministic chain: **PASS** — 21/21 checks on the sealed .so (transcript evidence).
- Task 3 restart recovery: **PASS** — same data root, statuses inspected via backend.
- Task 4 live-model chain: **PASS** — sensenova live credentials present; model/provider/platform/hashes recorded in verification.md.
- Task 5 boundary gate: **PASS** — dependency graph, loader manifest and package contents all clean; gate runs in CI.

GUI window lifecycle (open → close-hide → single-instance reopen → ordered quit): **PASS** on linux/amd64.

## Residual obligations

- `VIVYSHUTDOWN-DEADLINE-UTIL`: observed once — shutdown deadline exceeded while an in-flight utility generation (auto-title) held the host; retried clean. Accepted as transient for linux; a hard bound breach under load would reopen P0-A.
- Windows/amd64 native acceptance remains outstanding (`P0-NATIVE-VERIFICATION`) — the same driver + gate must run on the Windows runner before that platform is claimed.
- This acceptance covers P0-A only. P0-B closure still waits on DN-4/DN-6 domain evidence → DN-M.
