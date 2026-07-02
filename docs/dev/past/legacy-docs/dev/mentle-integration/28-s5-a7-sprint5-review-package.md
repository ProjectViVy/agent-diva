# Sprint 5 A7: Review Package and Sprint 6 Entry Baseline

## Purpose

This record closes Sprint 5 at the failure-mode and CI-hardening level.

It packages the Sprint 5 regression outputs so Sprint 6 can focus on release
candidate readiness and handoff rather than rediscovering downgrade semantics.

## Sprint 5 Scope Closed by This Package

Sprint 5 closes with these records:

- [26-s5-a1-failure-validation-matrix.md](./26-s5-a1-failure-validation-matrix.md)
- [27-s5-a2-a6-failure-and-ci-hardening.md](./27-s5-a2-a6-failure-and-ci-hardening.md)

Together they define the Sprint 6 starting contract for:

- Mentle open/startup failure fallback
- recall and diary-write failure semantics
- invalid dynamic tool definition handling
- runtime active state and prompt-routing consistency
- cron/default rebuild preservation
- `with_toolset()` registry-only prompt activation
- subagent default isolation
- default and Mentle feature CI coverage
- published `memtle 0.1.2` package source validation

## Sprint 5 Acceptance Matrix

| Acceptance item | Sprint 5 evidence | Review result |
|---|---|---|
| Mentle open failure degrades to Markdown memory | S5-F01 and S5-F02 tests cover override-unavailable and directory-create failure paths | Accepted |
| Prompt does not advertise unavailable Mentle capability | S5-F01, S5-F03, S5-F11, and S5-F13 cover inactive prompt routing | Accepted |
| Query failure is non-fatal | S5-F07 covers AgentLoop prefetch failure and core invalid-room failure | Accepted |
| Write failure does not break the main flow | S5-F08 and the AgentLoop consolidation sync-failure regression cover file-authoritative continuation | Accepted |
| Invalid tool definitions remain local skips | S5-F04 verifies valid names survive without relying on hard-coded counts | Accepted |
| Runtime active state matches actual tool assembly | S5-F03 and `test_runtime_active_matches_registered_status_tool` preserve the `memtle_status` anchor | Accepted |
| Cron/default rebuild preserves active Mentle tools | S5-F10 covers startup cron and later `register_default_tools(...)` rebuild | Accepted |
| `with_toolset()` remains registry-driven | S5-F11 and S5-F12 cover missing-status, status-present, and runtime isolation | Accepted |
| Subagents default to Mentle disabled | S5-F13 covers config, registry, and prompt isolation | Accepted |
| Default and Mentle build paths are continuously checked | S5-F14 and S5-F15 add local just recipes and CI hooks | Accepted |
| CI proves no local Mentle path dependency is used | S5-F16 moves package policy into a shared script used by CI and local commands | Accepted |

## Sprint 6 Entry Baseline

Sprint 6 may consume these assumptions as fixed unless a separate architecture
review reopens them:

- failed Mentle startup disables runtime-owned Mentle capability
- failed recall does not block the primary response path
- failed `memtle_diary_write` leaves file-backed history authoritative
- prompt routing remains anchored on assembled `memtle_status`
- `with_toolset()` stays external-registry driven
- subagents do not inherit parent Mentle tools by default
- CI covers default build, default assembly regressions, Mentle feature build,
  Mentle memory tests, Mentle agent tests, and the package source policy

## Verification Status for Review

Sprint 5 verification is recorded in:

- `docs/logs/2026-05-mentle-runtime/v0.0.5-s5-failure-and-ci-hardening/verification.md`

Recorded Sprint 5 validation:

- `cargo fmt --all -- --check`: passed
- `just sprint5-default-check`: passed
- `just mentle-package-policy`: passed
- `cargo check -p agent-diva-agent --features mentle`: passed after adding LLVM to PATH
- `cargo test -p agent-diva-agent --features mentle mentle`: passed
- `cargo test -p agent-diva-core --features mentle memory`: passed
- `just check`: passed
- `just test`: failed outside Sprint 5 scope because existing provider tests import
  `agent_diva_providers::ollama`, which is not exported by the current crate
  surface

## Open Risks Carried Forward

- Local Windows Mentle feature checks still require `clang-cl.exe` on PATH.
- Full workspace `just test` still has a pre-existing provider test export issue
  around `agent_diva_providers::ollama`.
- `with_toolset()` intentionally does not install a Mentle hybrid memory provider.
- Runtime hot-refresh of dynamic Mentle definitions remains out of scope.
- Sprint 7 GUI/tool-selection work must remain gated behind Sprint 6 RC
  acceptance.

## Review Outcome

Sprint 5 is accepted for Sprint 6 entry when reviewers agree that:

- all matrix rows have explicit test, CI, or documented evidence
- no failure path creates a prompt/tool mismatch
- local and CI validation commands cover both default and Mentle feature lanes
- remaining risks are operational and visible to Sprint 6
