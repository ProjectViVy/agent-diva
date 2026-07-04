# Wave 1 Review Summary

## Iteration

- Date: `2026-07-04`
- Version: `v0.0.1-wave1-summary`
- Scope:
  - `Wave A`
  - `Priority 1 / Wave B`
  - Commit range rooted in `94baa4b..HEAD` review program

## Overall Verdict

- `Wave A`: `Block release: yes`
- `Wave B`: `Block release: yes`
- Combined `Wave 1` verdict: `Do not ship`

## Top Risks

1. Budget enforcement is mostly unconnected to runtime hot paths.
2. Supervised-run timeout, cancellation, and ownership semantics are incomplete.
3. Security closure is not actually wired into production entrypoints.
4. E2E migration can report false greens because assertions and failures are downgraded.

## Wave A Summary

- Commits reviewed: `e3cd30c`, `3322aac`, `1627ea3`, `0183c3c`
- Key findings:
  - `P1`: `judge` assertions are skipped but counted as pass in the main E2E path.
  - `P1`: E2E runtime/network failures warn instead of failing by default, and the crate does not declare the `ci` feature it branches on.
  - `P2`: E2E runner bypasses manager runtime assembly and uses `AgentLoop::new + ToolConfig::default()`, creating fixture drift from production.
  - `P2`: `PokeEvent` claims an 8-stage lifecycle but the reviewed commit only emits 2 runtime variants.
  - `P2`: `ToolRegistry::execute()` now returns `ToolError`, but callers flatten it back into strings and lose error-category fidelity.

## Wave B Summary

- Commits reviewed: `45b6aa6`, `9242579`, `ce70902`, `7ca1c92`, `f43ff96`, `0c1d1bd`, `8ecf041`
- Key findings:
  - `P1`: Session token ledger and `token_budget_limit` are not connected to any runtime hot path.
  - `P1`: `per_task_token_budget` exists in `SubagentManager`, but all main construction paths pass `None`.
  - `P1`: `check_security()` is not wired into any production entrypoint.
  - `P1`: `check_security()` prioritizes PII sanitize ahead of injection block, allowing malicious content with PII to avoid blocking.
  - `P1`: `timeout_secs` and `RunExecutionError::{Timeout, Cancelled}` are data-only; the executor does not enforce them.
  - `P1`: Running supervised tasks continue after `cancelled` or `lost`, so DB terminal state can diverge from real execution.
  - `P2`: Subagent budget enforcement is a soft stop and can overshoot by a full LLM round.
  - `P2`: Subagent reported `token_usage` keeps only the last iteration usage instead of cumulative task usage.
  - `P2`: Supervised run state helpers and `requeue()` semantics disagree.
  - `P2`: Security audit coverage is partial; core `check_security()` decisions do not emit the new security audit events.

## Cross-Wave Matrix

- Security:
  - `check_security()` not wired to production
  - PII short-circuits injection block
  - `SkillLoaded` audit semantics overstate trust
- Budget / usage:
  - session ledger path is dead
  - subagent per-task budget is configured but never enabled
  - task usage reporting undercounts multi-iteration runs
- Concurrency / async:
  - supervised tasks continue running after `cancelled/lost`
  - timeout/cancel semantics are not enforced
- Error propagation:
  - `ToolError` becomes plain text at caller boundaries
  - supervised executor swallows invalid-state completion/failure fallout into logs
- Test integrity:
  - `judge` assertions false-pass
  - E2E failure paths warn instead of fail
  - E2E fixture does not match real runtime assembly
- Contract / state-machine:
  - `PokeEvent` lifecycle contract overclaims real coverage
  - `can_transition_to()` conflicts with `requeue()`

## Priority Pool

### P1

- `0183c3c`: `judge` assertions false-pass in main E2E path
- `0183c3c`: E2E failures can downgrade to warnings and still pass
- `45b6aa6`: session ledger / `token_budget_limit` not wired
- `9242579`: `per_task_token_budget` never enabled by main construction paths
- `ce70902`: supervised timeout/cancel semantics are declarative only
- `ce70902`: cancelled/lost supervised tasks keep executing
- `f43ff96` / `0c1d1bd`: `check_security()` not wired into production
- `f43ff96` / `0c1d1bd`: PII sanitize short-circuits injection block

### P2

- `0183c3c`: E2E fixture drift from manager runtime assembly
- `e3cd30c`: `PokeEvent` lifecycle contract incomplete
- `3322aac`: `ToolError` category fidelity lost at caller boundary
- `9242579`: subagent budget is soft stop, not hard limit
- `9242579`: subagent cumulative usage is underreported
- `ce70902`: `can_transition_to()` conflicts with `requeue()`
- `ce70902`: supervised audit misses cancel/requeue and lacks budget/security hooks
- `f43ff96` / `0c1d1bd`: `check_security()` decisions do not emit core audit events
- `f43ff96` / `0c1d1bd`: `SkillLoaded` audit trust semantics are misleading

## Recommended Fix Order

1. Repair false-green test infrastructure first:
   - `0183c3c` judge assertions
   - `0183c3c` fail-on-runtime-error behavior
2. Make budget claims true:
   - `45b6aa6` session ledger wiring
   - `9242579` subagent budget enablement
3. Close real runtime safety gaps:
   - `ce70902` timeout/cancel/ownership semantics
   - `f43ff96` production security wiring
4. Clean up fidelity and observability gaps:
   - `3322aac` structured tool errors
   - `e3cd30c` lifecycle contract
   - security/supervised audit coverage
