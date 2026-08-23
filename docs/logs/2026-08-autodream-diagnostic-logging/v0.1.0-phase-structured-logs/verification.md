# Verification

## Commands

| 命令 | 结果 |
| --- | --- |
| `cargo fmt -p agent-diva-autodream -- --check` | **pass** |
| `cargo clippy -p agent-diva-autodream --all-targets -- -D warnings` | **pass** |
| `cargo test -p agent-diva-autodream -- --test-threads=1` | **40 passed** (16 lib + 24 integration, including 4 new diagnostics tests) |

Did not run full-workspace `just ci`: this iteration only changes the
AutoDream crate's diagnostic surface.

## 测到了什么

- Success path emits `phase_started` for orient/gather/consolidate/propose,
  `input_collected` with a compact input summary, and `worker_succeeded`.
- Propose path logs `skill_candidate_rejected` with `gate_code=skill_slug_invalid`
  and `skill_request_created` with `proposal_id`.
- Gather failure logs `failure_code=input_unavailable`.
- Missing MemoryHome logs `memory_home_missing` with `phase=orient`.
- Default S3 without a Skill engine logs `skill_reflection_degraded` with
  `failure_code=provider_unavailable`; InvalidSchema maps to `invalid_candidate`.
- Old five-field JSONL events still deserialize.
- Existing S3 tests still assert zero BML database file and zero memory
  proposals.

## 测不到什么

- Live `tracing` subscriber output in production (JSONL is the durable
  complete log; tracing shares the same helper).
- GUI event timeline rendering of the new optional fields.
- Real LLM Skill reflection.
