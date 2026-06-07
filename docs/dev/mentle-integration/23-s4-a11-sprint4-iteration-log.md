# Sprint 4 A11: Iteration Log

## Purpose

S4-A11 consolidates Sprint 4 summary, verification, acceptance, and release
notes so the Sprint 4 architecture sign-off can consume one stable evidence
chain.

This record summarizes the iteration logs at:

- `docs/logs/2026-05-mentle-runtime/v0.0.2-s4-agentloop-hardening/`
- `docs/logs/2026-05-mentle-runtime/v0.0.3-s4-a8-compatibility-review/`
- `docs/logs/2026-05-mentle-runtime/v0.0.4-s4-regression-and-env/`

## Summary

Sprint 4 closes the AgentLoop hardening scope by proving that Mentle prompt
routing follows actual tool availability across initial assembly, cron/default
rebuild, `with_toolset()`, and subagent paths.

Sprint 4 also closes the compatibility review against the Sprint 3 adapter and
runtime baseline. No break was found in dynamic tool definitions, `call_json`
execution, toolkit error mapping, invalid-definition skipping, or
`memtle_status`-anchored activation.

## Verification

The Sprint 4 verification chain includes:

- default formatting and default-lane compilation
- default-lane assembly, cron/default rebuild, `with_toolset()`, subagent config,
  subagent registry, and subagent prompt regression tests
- Mentle feature agent checks and tests after adding LLVM to the current shell
  PATH on Windows
- Mentle feature core memory tests
- static policy checks that `memtle 0.1.2` is registry-sourced and not overridden

## Acceptance

Sprint 4 is accepted when:

- `build_agent_tools(...)` remains the single registry assembly helper
- initial assembly and rebuild paths preserve runtime custom tools
- `with_toolset()` derives Mentle prompt routing only from supplied registry
  contents
- subagents do not inherit Mentle long-term memory capability or parent custom
  tools by default
- adapter/runtime compatibility with Sprint 3 remains accepted
- Mentle feature verification is recorded as passed or explicitly blocked by
  host prerequisites

## Release

No standalone release action is required for S4-A9 through S4-A12.

The Rust test additions and documentation updates will ship with the normal
workspace release flow. The local Windows PATH note is an operator/developer
environment clarification, not a runtime behavior change.

## Open Follow-ups

- Sprint 5 should continue with failure modes and CI hardening if deeper
  failure-injection coverage is needed.
- The unrelated local full-lib observation
  `skills::tests::test_default_builtin_dir_loads_skills` remains outside the
  Sprint 4 Mentle assembly scope.
