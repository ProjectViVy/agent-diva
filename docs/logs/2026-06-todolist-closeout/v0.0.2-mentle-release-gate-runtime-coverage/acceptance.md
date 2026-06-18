# Mentle Release-Gate Runtime Coverage Acceptance

## Acceptance Steps

1. Run `cargo test -p agent-diva-agent --test mentle_governance_boundaries`.
2. Confirm both tests pass.
3. Inspect `agent-diva-agent/tests/mentle_governance_boundaries.rs` and confirm coverage constructs enabled Mentle runtime toolsets rather than only asserting `MentleToolRuntimeConfig` filtering.
4. Inspect `TODOLIST.md` and confirm the Mentle release-gate coverage item is under Done.

## Expected Result

The Story 6.5 release-gate coverage fails if enabled Mentle runtime surfaces reintroduce default governance recall/routing into prompt assembly.
