# E5 Verification

## Focused automated checks

- `cargo test -p agent-diva-laputa --test feedback`
  - AutoDream provenance preserved.
  - feedback delayed until terminal outcome.
  - successful and failed/corrected outcomes recorded without payload.
  - governed deprecation adapts to a content-free tombstone.
- `cargo test -p agent-diva-autodream --test feedback_inputs`
  - bounded collector prioritizes payload-free Recall feedback.
- `cargo test -p agent-diva-autodream --test feedback_reflection`
  - corrected feedback creates a governed deprecation candidate.
- `cargo test -p agent-diva-autodream --test candidates`
  - candidate policy regressions pass.
- `cargo test -p agent-diva-agent test_agent_loop_prefetch --lib`
  - live Recall boundary regression passes.
- `cargo check -p agent-diva-manager`
  - Manager, AgentLoop, typed provider and apply route compile together.

## Slice gates

`just fmt-check` and `just check` are required before commit. Full `just test`
remains the E7 release gate because the active Windows desktop binary and the
MSVC GUI test PDB limitation are already tracked as release blockers.

No external API, desktop key, user Memory content, or manual desktop test was
used.
