# GMH-22 Verification

## Focused coverage

- `cargo test -p agent-diva-core recall`: passed, 11 matching tests.
- `cargo test -p agent-diva-core --features mentle mentle_search_hit_cannot_self_promote_to_prompt_authority`: passed, 1 test.
- `cargo check -p agent-diva-core --features mentle`: passed.

Fixtures cover stable JSON, all filter classes, candidate limits,
supersession/deduplication, deterministic score/decay/tie-breaks, zero/exact
and competing budgets, Unicode and wrapper cost, no truncation, retrieval
degradation, malicious content escaping, content-free trace/shadow output, and
Mentle trust isolation.

## Workspace gates

- `cargo test -p agent-diva-core memory`: passed, 33 matching tests.
- `cargo test -p agent-diva-laputa memory_provider`: passed, 4 tests.
- `cargo test -p agent-diva-agent prefetch`: passed, 6 tests.
- `git diff --check`: passed.
- `just fmt-check`: passed.
- `just check`: passed.
- `just test`: passed for the complete workspace.

No CLI/GUI or real-device smoke is required because Recall v2 is not registered
on the production prefetch path. GMH-24 will trigger the recorded real-device
reminder before shadow/cutover acceptance.
