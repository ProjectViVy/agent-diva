# C6 production cutover verification

## Passed during implementation

- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p agent-diva-core config::validate --lib` — 6 passed
- `cargo test -p agent-diva-core --test channel_protocol_tck` — 5 passed
- `cargo test -p agent-diva-channels --all-targets` — 153 passed, QQ live harness ignored
- Manager focused Fabric, attachment, route-removal, and runtime-update tests
- `pnpm test` — 76 files, 533 tests passed
- `pnpm build` — TypeScript and Vite production build passed
- `python scripts/ci/check_channel_clean_break.py --self-test`
- `python scripts/ci/check_channel_clean_break.py`
- `cargo run -p agent-diva-cli -- --help` — executable CLI smoke passed
- `cargo test --all -- --quiet` — final full workspace unit, integration, and doc-test gate passed

The full-suite stabilization passes exposed three assumptions that focused tests did not cover: a
CLI integration test still targeted the removed chat route, websocket notifications could precede
the matching response, and SQLite sidecars could appear while a context test enumerated files. The
CLI test now exercises the typed runtime-turn route, and the two timing-sensitive assertions accept
the valid runtime order/state. The Manager projection test also waits for both ordered live events
before reading the journal. The final full workspace gate passed after these corrections.

## Post-merge validation on `dev`

- Merge commit: `8cc6580b` (`merge: integrate C6 channel cutover into dev`)
- `just fmt-check` — passed
- `just check` — passed
- `just channel-clean-break-check` — self-test and production scan passed
- `just test` — full workspace unit, integration, and doc-test gate passed

The only merge conflict was `LOCK.md`; resolution retained both the released implementation record
and the active merge-validation lock. No product-source conflict occurred.

## Deferred exit criteria

- Full workspace Rust 1.80 probe failed before compilation because Tauri's build dependency chain
  resolves `getrandom 0.4.3`, whose Edition 2024 manifest cannot be parsed by Cargo 1.80.1. The
  channel-scoped `just msrv-probe check -p agent-diva-channels` passed. The broader pre-existing
  dependency conflict remains tracked by `WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS`.
- Post-cutover real Tauri disconnect/resume acceptance.
- At least one real external platform ingress + final delivery receipt.
- Physical removal of AgentLoop `InboundMessage`/`OutboundMessage`.

These are tracked as C6-D/C6-E in `TODOLIST.md`; therefore this record must not be interpreted as
the complete C6 release gate.
