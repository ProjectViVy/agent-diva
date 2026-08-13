# Verification

## Commands

```text
cargo test -p agent-diva-agent --features mentle --lib open_isolation_tests -- --nocapture
cargo test -p agent-diva-agent --features mentle --lib mentle -- --nocapture
cargo build -p agent-diva-cli
target\debug\agent-diva.exe gateway run   # smoke, ~15s then stop
```

## Results

| Check | Result |
|-------|--------|
| `open_isolation_tests` (4) | pass |
| mentle lib filter (32) | 31 pass / 1 fail (`test_register_default_tools_rebuild_keeps_active_mentle_prompt`, pre-existing prompt boundary) |
| Gateway smoke | process stayed up; no `STATUS_STACK_OVERFLOW` |
| Mentle assemble log | `Mentle runtime assembled on isolated large-stack thread ... tool_count=32` |
| Ready log | `Gateway ready; HTTP API at http://127.0.0.1:3000` |

## Notes

- First partial fix (open-only isolation) still overflowed during `HybridMemoryProvider` warm-up on the main stack; full assemble isolation fixed it.
- Unrelated QQ channel auth failure appeared in smoke logs and does not block gateway readiness.
