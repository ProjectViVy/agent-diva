# HQ-05 Verification

## Final release gates

- `just fmt-check`: passed.
- `just check`: passed with Clippy warnings denied across the workspace.
- `just test`: passed across the complete Rust workspace and doc tests.
- `pnpm test` in `agent-diva-gui`: 73 files and 516 tests passed.
- `pnpm build` in `agent-diva-gui`: Vue typecheck and production Vite build passed. Existing large
  chunk notices remain informational.

## Focused session-admission regressions

- Core configuration defaults/validation: 2 passed.
- Core admission kernel: 12 passed.
- Agent Bus/direct characterization and fault injection: 6 passed.
- Agent dispatcher FIFO/control matrix: 7 passed.
- Manager same-chat request isolation: 1 passed.
- CLI `update_plan_e2e`: 1 passed.
- GUI active-request admission router: 3 passed.
- Tauri `queued_preserved` Stop outcome: 1 passed.
- Embedded Gateway `/api/health`: 1 passed.

## Minimum real-path smoke

- `cargo run -p agent-diva-cli -- agent --help`: passed and rendered the direct-agent CLI contract.
- `pnpm dev -- --host 127.0.0.1 --port 4173 --strictPort`: Vite became ready; HTTP GET `/` returned
  200 and contained the application root. The process was then stopped with Ctrl-C.
- `cargo test -p agent-diva-gui embedded_gateway_serves_health_endpoint --lib -- --nocapture`:
  passed against an ephemeral embedded-Gateway port.

Two attempted hidden `Start-Process` smoke wrappers were rejected by local command policy before a
process was created. The controlled foreground pipe smoke above replaced them successfully.

## Non-blocking existing notices

- Test-only `agent-diva-core` unused-variable warnings remain unchanged; workspace Clippy with
  `-D warnings` passed.
- Cargo continues to report the existing `imap-proto 0.10.2` future-incompatibility notice. No HQ-05
  release gate failed.
