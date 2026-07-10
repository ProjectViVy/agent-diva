# Acceptance

1. On Windows, run `just diva-gate` or `cargo run -p agent-diva-cli -- gateway run` with Mentle enabled (default Full).
2. Confirm bootstrap reaches `Gateway ready` without `STATUS_STACK_OVERFLOW`.
3. Confirm logs include Mentle runtime assembly with a non-zero `tool_count` (e.g. 32).
4. Confirm Laputa injection (if `.laputa` exists) still works alongside Mentle tools.
5. Optional: call `GET /api/tools/mentle/available` and verify `memtle_*` names are listed.
