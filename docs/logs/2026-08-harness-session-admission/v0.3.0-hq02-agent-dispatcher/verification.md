# HQ-02 Verification

## Focused coverage

- Dispatcher: 6 tests passed for same-session FIFO/no reentry, cross-session overlap, queue-full and timeout zero-execution boundaries, Stop, and Reset.
- Agent crate: 426 unit tests plus compaction, multimodal, world-routing, and doc tests passed after the ownership refactor.
- Tools crate: 124 unit tests passed, including inherited background-task mask metadata.
- `cargo clippy -p agent-diva-agent -p agent-diva-tools --lib -- -D warnings` passed.

## Workspace gates

- `just fmt-check`: passed.
- `just check`: passed.
- `just test`: passed. Existing non-failing test warnings and the upstream `imap-proto` future-incompatibility notice remain unchanged.

## Smoke

- `just run agent --help`: passed and rendered the expected direct-agent CLI contract.
- An initial `just run -- agent --help` invocation failed because the extra separator became a literal CLI argument; rerunning with the recipe's correct syntax passed.

## Observed invariants

- Queue rejection occurs before the execution future is constructed or polled.
- Dispatcher/admission mutexes are released before turn execution awaits.
- Stop leaves waiters queued; Reset/Delete cancel waiters with `SessionReset`.
- No MessageBus or public event wire shape changed; production dispatch remains compatibility-serial until HQ-03 correlation.
