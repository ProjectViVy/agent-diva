# Sprint 3 A8: Review Package and Sprint 4 Entry Baseline

## Purpose

This record closes Sprint 3 at the interface and verification-packaging level.
It packages the Sprint 3 adapter/runtime outputs so Sprint 4 can consume a
stable baseline for AgentLoop, cron rebuild, and external toolset behavior.

This is a review and handoff document. It does not introduce new Rust behavior.

## Sprint 3 Scope Closed by This Package

Sprint 3 closes with these frozen records:

- [13-s3-a1-memtle-toolkit-tool-interface.md](./13-s3-a1-memtle-toolkit-tool-interface.md)
- [14-s3-a2-dynamic-tool-registration-model.md](./14-s3-a2-dynamic-tool-registration-model.md)
- [15-s3-a3-toolkit-error-mapping.md](./15-s3-a3-toolkit-error-mapping.md)
- [16-s3-a4-a6-mentle-runtime-assembly.md](./16-s3-a4-a6-mentle-runtime-assembly.md)
- [17-s3-a7-test-and-verification-baseline.md](./17-s3-a7-test-and-verification-baseline.md)

Together they define Sprint 4's starting contract for:

- dynamic Mentle tool exposure
- runtime ownership and assembly boundaries
- prompt-routing activation rules
- minimum regression and verification expectations

## Sprint 3 Acceptance Matrix

| Acceptance item | Sprint 3 baseline evidence | Review result |
|---|---|---|
| Tool adapter contract is frozen | S3-A1 records metadata, schema, shared toolkit handle, and `call_json` execution path | Ready for review |
| Dynamic registration is frozen | S3-A2 records `tool_definitions()` as the only schema source and reusable `Vec<Arc<dyn Tool>>` output | Ready for review |
| Error mapping is unified | S3-A3 records startup failure, invalid-definition skip, and tool-call error exposure rules | Ready for review |
| Runtime assembly boundary is frozen | S3-A4-A6 records `MentleRuntime` ownership, active flag rule, and helper interface | Ready for review |
| Verification minimum is explicit | S3-A7 records default lane, dynamic tools, inactive runtime, rebuild-path, and Mentle lane expectations | Ready for review |
| Sprint 4 prompt/tool gating baseline is explicit | `memtle_status` remains the activation anchor for prompt routing | Ready for review |

## Sprint 4 Entry Baseline

Sprint 4 must consume the following assumptions as fixed unless a new
architecture review reopens them:

- `MentleRuntime` is the only internal runtime ownership boundary for toolkit,
  provider, custom tools, and active state
- `MemtleToolkit::tool_definitions()` is the only dynamic tool schema source
- `MemtleToolkit::call_json(name, args).await` is the only generic adapter
  execution path
- prompt routing is driven by actual registered tool availability, not by Cargo
  feature enablement or toolkit-open success alone
- `memtle_status` is the activation anchor for `with_mentle(true)`
- custom Mentle tools are reusable startup state and must survive rebuild paths
- invalid tool definitions are skipped individually and do not deactivate an
  otherwise-open runtime
- explicit injected memory providers are not overridden by the runtime provider

## Sprint 4 Work That May Assume This Baseline

Sprint 4 can proceed without re-deciding:

- AgentLoop startup assembly behavior
- cron rebuild preservation behavior
- `with_toolset()` prompt/tool consistency rule
- inactive Mentle prompt-routing behavior

Sprint 4 still needs to implement or harden:

- deeper AgentLoop integration on top of the frozen runtime helper
- advanced entrypoint coverage around external registry usage
- subagent isolation and related regression coverage

## Open Risks Carried Forward

The following risks remain open after Sprint 3 and must be visible to Sprint 4:

- Windows Mentle feature-lane verification is blocked on native toolchain
  availability, specifically `clang-cl.exe`
- published `memtle 0.1.2` upstream schema or native dependency changes remain a
  future upgrade risk
- Sprint 4 still needs end-to-end confirmation that all advanced assembly paths
  preserve prompt/tool consistency under real integration wiring

## Verification Status for Review

Recorded Sprint 3 verification interpretation:

- `cargo fmt`: passed
- `cargo fmt --all -- --check`: passed
- `cargo check -p agent-diva-agent --no-default-features`: passed
- `cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron`: passed
- `cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron`: passed
- `cargo check -p agent-diva-agent --features mentle`: blocked by missing `clang-cl.exe`
- `cargo test -p agent-diva-agent --features mentle mentle`: blocked by missing `clang-cl.exe`
- `cargo test -p agent-diva-core --features mentle memory`: blocked by missing `clang-cl.exe`

The current Sprint 3 review package is therefore complete for interface and
default-lane verification, with one explicit environment block carried into
Sprint 4.

## Review Outcome Template

Sprint 3 may be accepted for Sprint 4 entry when reviewers agree that:

- Sprint 3 interface assumptions are fully documented
- Sprint 4 no longer needs to invent temporary test standards
- remaining risk is explicit and operational rather than architectural
