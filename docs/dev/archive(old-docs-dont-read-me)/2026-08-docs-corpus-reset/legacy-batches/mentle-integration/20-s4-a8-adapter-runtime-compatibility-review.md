# Sprint 4 A8: Adapter/Runtime Compatibility Review

## Purpose

S4-A8 closes the Sprint 4 compatibility review owned by A-MEM. It checks that
the AgentLoop hardening work from S4-A2 through S4-A6 did not break the frozen
Sprint 3 adapter/runtime conventions for Mentle tool definitions, `call_json`
execution, and toolkit error mapping.

This is a review record. It does not introduce new Rust behavior.

## Baseline Under Review

The review consumes these frozen Sprint 3 contracts:

- [13-s3-a1-memtle-toolkit-tool-interface.md](./13-s3-a1-memtle-toolkit-tool-interface.md)
- [14-s3-a2-dynamic-tool-registration-model.md](./14-s3-a2-dynamic-tool-registration-model.md)
- [15-s3-a3-toolkit-error-mapping.md](./15-s3-a3-toolkit-error-mapping.md)
- [16-s3-a4-a6-mentle-runtime-assembly.md](./16-s3-a4-a6-mentle-runtime-assembly.md)
- [18-s3-a8-sprint3-review-package.md](./18-s3-a8-sprint3-review-package.md)

It reviews the Sprint 4 assembly paths recorded in:

- [19-s4-a1-sprint4-entry-audit.md](./19-s4-a1-sprint4-entry-audit.md)

## Compatibility Matrix

| Contract | Current evidence | Result |
|---|---|---|
| Tool schemas are sourced only from `MemtleToolkit::tool_definitions()` | `MentleRuntime::try_build(...)` opens the toolkit once, reads `tool_definitions()`, and maps the resulting definitions through `mentle_tools_from_definitions(...)` | Compatible |
| Adapter consumes only `name`, `description`, and `inputSchema` | `mentle_tool_metadata_from_definition(...)` reads those three fields and rejects missing/non-object schema data | Compatible |
| Provider schema uses the generic `Tool::to_schema()` path | `MentleToolkitTool::parameters()` returns the cloned `inputSchema`; tests assert the generated schema keeps `function.parameters == inputSchema` | Compatible |
| Tool execution goes only through `MemtleToolkit::call_json(name, args).await` | `MentleToolkitTool::execute(...)` locks the shared toolkit and calls `call_json(&self.name, args)` directly | Compatible |
| Toolkit call failures map to `ToolError::ExecutionFailed` | Transport errors and payload errors are both mapped through `mentle_execution_failed(...)` | Compatible |
| Invalid definitions are skipped individually | Definition mapping returns `None` for invalid definitions and logs `fallback_action = "skip_tool"` without deactivating an otherwise-open runtime | Compatible |
| Runtime activation remains anchored on `memtle_status` | `MentleRuntime::from_parts(...)` derives `active` from the custom tool vector; `AgentLoop` and `with_toolset()` route prompts from actual registry/runtime availability | Compatible |
| S4-A2/S4-A6 assembly paths preserve S3 custom tool conventions | Initial assembly, cron rebuild, default-tool rebuild, and `with_toolset()` consume the same custom-tool registry path without rediscovering Mentle tools | Compatible |

## Review Findings

No compatibility break was found.

The current code still preserves the Sprint 3 boundaries:

- no hard-coded full Mentle tool list
- no local `memtle` path/git/patch override
- no adapter-side argument filling or business-semantic rewrite
- no public Agent-Diva API widened with Mentle-specific toolkit error types
- no prompt routing based only on feature/config enablement

## Verification

Commands and checks run for this review:

```bash
cargo fmt --all -- --check
cargo check -p agent-diva-agent --no-default-features
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-agent test_with_toolset_memtle_tool_without_status_disables_prompt
cargo test -p agent-diva-agent test_tool_assembly_subagent_mode_excludes_mentle_custom_tools
cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron
cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron
cargo test -p agent-diva-agent --features mentle test_tool_definitions_are_dynamic
cargo test -p agent-diva-agent --features mentle test_mentle_tool_adapter_executes_json_call
cargo test -p agent-diva-agent --features mentle test_mentle_tool_adapter_translates_call_errors
cargo test -p agent-diva-agent --features mentle test_mentle_tool_adapter_translates_payload_error
cargo test -p agent-diva-agent --features mentle test_mentle_tool_registration_skips_invalid_definition_only
cargo test -p agent-diva-agent --features mentle test_with_tools_active_runtime_enables_registry_and_prompt
cargo test -p agent-diva-agent --features mentle test_mentle_runtime_without_status_disables_prompt_routing
```

Static policy checks also confirmed:

- `Cargo.lock` resolves `memtle 0.1.2` from the crates.io registry
- no `memtle` path dependency, git dependency, or `[patch.crates-io]` override is present in workspace manifests

All checks passed.

## Outcome

S4-A8 is accepted. Sprint 4 adapter/runtime compatibility remains aligned with
the Sprint 3 `tool_definitions()`, `call_json`, and error-mapping conventions.
