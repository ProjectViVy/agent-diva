# Verification

- Reproduced the original failure with `cargo run -p agent-diva-cli -- gateway run`.
- Confirmed the native crash stack through GDB at `simsimd_wsum_u8`.
- Added a Windows-only runtime guard in `agent-diva-agent/src/mentle_runtime.rs`.
- `cargo fmt --all -- --check` passed.
- `cargo test -p agent-diva-agent --features mentle --lib mentle_runtime -- --nocapture` passed (2 tests).
- Gateway smoke startup reached `Gateway ready; HTTP API at http://127.0.0.1:3000` and logged the Windows Mentle fallback; no stack overflow occurred.
