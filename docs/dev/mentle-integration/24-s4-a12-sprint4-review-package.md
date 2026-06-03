# Sprint 4 A12: Review Package and Architecture Sign-off

## Purpose

This record closes Sprint 4 at the regression, environment, and architecture
sign-off level.

It packages the AgentLoop hardening outputs so later failure-mode and release
work can consume a stable baseline for initial assembly, cron/default rebuild,
`with_toolset()`, subagent isolation, prompt routing, and Mentle feature-lane
verification.

This is a review and handoff document. It does not introduce new runtime
behavior.

## Sprint 4 Scope Closed by This Package

Sprint 4 closes with these frozen records:

- [19-s4-a1-sprint4-entry-audit.md](./19-s4-a1-sprint4-entry-audit.md)
- [20-s4-a8-adapter-runtime-compatibility-review.md](./20-s4-a8-adapter-runtime-compatibility-review.md)
- [21-s4-a9-regression-test-baseline.md](./21-s4-a9-regression-test-baseline.md)
- [22-s4-a10-mentle-feature-build-env.md](./22-s4-a10-mentle-feature-build-env.md)
- [23-s4-a11-sprint4-iteration-log.md](./23-s4-a11-sprint4-iteration-log.md)

Together they define the Sprint 5 starting contract for:

- shared registry assembly through `build_agent_tools(...)`
- custom tool preservation across startup and cron/default rebuilds
- `with_toolset()` registry-only prompt activation
- subagent isolation at config, registry, and prompt-template levels
- Windows Mentle feature-lane prerequisite recording

## Sprint 4 Acceptance Matrix

| Acceptance item | Sprint 4 evidence | Review result |
|---|---|---|
| Initial AgentLoop assembly consumes active runtime state | S4-A1 records `MentleRuntime` helper consumption; v0.0.2 records `test_with_tools_active_runtime_enables_registry_and_prompt` | Accepted |
| `build_agent_tools(...)` remains the single registry assembly helper | S4-A1 and S4-A9 record shared assembly and cron/default rebuild tests | Accepted |
| Cron/default rebuild preserves custom tools | S4-A9 records `test_build_agent_tools_reuses_custom_tools_with_cron` and `test_register_default_tools_preserves_custom_tools_with_cron` | Accepted |
| `with_toolset()` does not create prompt/tool split | S4-A9 records missing-status, non-status-tool, status-present, and construction-time external-registry isolation tests | Accepted |
| Subagents do not inherit parent tool capability by default | S4-A9 records config, registry, and prompt-template isolation tests; registry isolation drops parent custom tools rather than filtering Mentle tools only | Accepted |
| Sprint 3 adapter/runtime contracts remain compatible | S4-A8 records no compatibility break against `tool_definitions()`, `call_json`, error mapping, and invalid-definition skip behavior | Accepted |
| Mentle feature lane is reproducible on this Windows host | S4-A10 records Rust `1.93.0`, LLVM `clang-cl.exe`, PATH prefix, and passed Mentle checks | Accepted |
| Package source policy remains intact | v0.0.4 records `memtle 0.1.2` registry dependency and no `path`, `git`, or `[patch.crates-io]` override | Accepted |

## Architecture Sign-off

Sprint 4 is accepted with no known prompt/tool split across the reviewed
assembly paths.

The signed-off invariants are:

- `MentleRuntime` remains the internal owner of toolkit, memory provider,
  reusable custom tools, and active state.
- `build_agent_tools(...)` remains the shared registry assembly path for main
  AgentLoop startup and rebuild behavior.
- `memtle_status` remains the activation anchor for Mentle prompt routing.
- `with_toolset()` derives `mentle_active` from the supplied registry and does
  not construct Mentle runtime state.
- Subagents default to Mentle disabled and do not receive parent Mentle custom
  tools or prompt-routing text.
- Subagent registry assembly intentionally does not inherit parent custom tools.
- Sprint 4 does not introduce runtime hot-refresh of Mentle tool definitions;
  `MentleRuntime` active state is a construction-time snapshot.

## Verification Status for Review

Recorded Sprint 4 closure verification:

- `cargo fmt --all -- --check`: passed
- `cargo check -p agent-diva-agent --no-default-features`: passed
- `cargo test -p agent-diva-agent test_with_toolset`: passed
- `cargo test -p agent-diva-agent test_tool_assembly_subagent_mode_excludes_mentle_custom_tools`: passed
- `cargo test -p agent-diva-agent subagent_does_not_receive_mentle_by_default`: passed
- `cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron`: passed
- `cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron`: passed
- `cargo test -p agent-diva-agent test_build_subagent_prompt_omits_mentle_routing`: passed
- `cargo check -p agent-diva-agent --features mentle`: passed after adding LLVM to PATH
- `cargo test -p agent-diva-agent --features mentle mentle`: passed
- `cargo test -p agent-diva-agent --features mentle test_with_tools_active_runtime_enables_registry_and_prompt`: covered by the Mentle feature lane and previously recorded in S4-A8
- `cargo test -p agent-diva-agent --features mentle test_with_tools_startup_cron_preserves_mentle_custom_tools`: covered by the Mentle feature lane
- `cargo test -p agent-diva-core --features mentle memory`: passed
- static package-source policy checks: passed

## Environment Note

This Windows host has `clang-cl.exe` at:

```text
C:\Program Files\LLVM\bin\clang-cl.exe
```

The current shell did not initially find it through PATH. Mentle feature
verification passed after this session-level prefix:

```powershell
$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH
```

## Open Risks Carried Forward

- Sprint 5 may still add deeper failure-injection and CI-hardening coverage.
- Local developer shells can still report Mentle feature builds as blocked if
  LLVM is installed but not present in PATH; S4-A10 defines the reproducible
  fix and blocked-recording rule.
- The unrelated local full-lib observation
  `skills::tests::test_default_builtin_dir_loads_skills` remains outside the
  Mentle assembly scope and should not block this review package.

## Review Outcome

Sprint 4 is accepted for handoff when reviewers agree that:

- A9 regression evidence covers all advanced assembly paths listed above
- A10 environment evidence is reproducible and no longer ambiguous
- A11 documentation and iteration logs are complete
- no reviewed path advertises Mentle prompt capability without matching tool
  availability
