# Verification

- `cargo test -p agent-diva-agent --features mentle --lib tool_rebuilds -- --nocapture` — passed, 2 tests.
- `cargo test -p agent-diva-agent --features mentle --lib mentle` — passed, 33 tests.
- `cargo test -p agent-diva-agent --lib` — passed, 361 tests.
- `cargo fmt --all -- --check` — passed.

Windows Mentle commands run with `C:\Program Files\LLVM\bin` prepended to `PATH` so `clang-cl.exe` is available.
