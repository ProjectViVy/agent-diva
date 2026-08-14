# I1-S3 Memory / ACTMEM / Recap — Verification

## Automated results

- `just fmt-check` — passed.
- `just check` — passed with warnings denied.
- `just bml-boundary-check` — passed after locating `MemoryHome` in the BML logical layer.
- Focused MemoryHome, ACTMEM, Agent lifecycle/tool, AutoDream, and Manager HTTP tests — passed.
- `pnpm test` in `agent-diva-gui` — 63 files, 467 tests passed.
- `pnpm build` in `agent-diva-gui` — Vue type-check and Vite production build passed; only existing large-chunk advisories were emitted.
- `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml` — passed.
- `cargo run -p agent-diva-cli -- --help` — passed as the minimum executable CLI smoke.
- `just test` — passed across the full Rust workspace, integration suites, and doctests.

## Covered invariants

- Lazy config-home paths, no legacy import, LongTerm-only writes, record CAS/tombstones, evidence round-trip, MEMRULES defaults, and checkpoint startup GC.
- Strict ACTMEM parse/no-op/CAS/caps, capsule chunking and safe names.
- Immediate Pulse/Recap, ten-minute idle folding, generation cancellation, cron/subagent exclusion, and reset cleanup.
- CORE/DEFER partitioning, subagent exclusion, and MEMRULES visibility before a write tool call.
- AutoDream Work organization, allowed retry classification, Work conflict failure, and zero Memory proposals/BML writes.
- Manager CRUD/error independence and GUI direct-edit/no-approval flows.

## Deferred native smoke

The current automation can compile the Tauri application but cannot drive the native WebView to perform stateful desktop interactions. `MEMORY-S3-DESKTOP-SMOKE` remains open and UI-S3 is not marked complete until a human completes the acceptance scenarios.
