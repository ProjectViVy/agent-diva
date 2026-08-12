# Unified Memory Governance Verification

## Passed

- `cargo test -p agent-diva-laputa --test governed_apply --test service`: 14 passed.
- `cargo test -p agent-diva-manager handlers::laputa::recovery_tests --lib`: 11 passed.
- `cargo test -p agent-diva-manager --test autodream_laputa_e2e`: 6 passed.
- `cargo test -p agent-diva-autodream`: passed.
- `cargo check -p agent-diva-laputa -p agent-diva-agent -p agent-diva-manager`: passed.
- `just fmt-check`: passed.
- `just check`: passed.
- `just ci` with an isolated Cargo target: passed, including the full workspace tests, benchmark,
  feature-gate, Laputa clean-break, and BML boundary gates.

The regressions prove that injected governance is the only event authority, a missing pending
request is restored once, legacy allowed receipts are not imported, GET projections append no
ledger events, and JSON `null` sections are reported as `tbd`.

## Runtime smoke

The first workspace `just test` attempt could not replace `target/debug/agent-diva.exe` because
the user's active desktop smoke process held the executable (`os error 5`). No process was killed.
The final CI gate then passed from an isolated target. After the code gates completed, the old
Gateway was stopped by exact PID and restarted from the current branch. `/api/health` returned
200/ready; Persona and Evolution each returned five proposals with five governance projections
and zero missing projections. Startup reconciled five prior pending requests.
