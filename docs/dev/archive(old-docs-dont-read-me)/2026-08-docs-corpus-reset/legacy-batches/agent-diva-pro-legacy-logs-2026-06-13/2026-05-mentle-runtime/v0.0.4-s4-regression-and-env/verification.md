# Verification

## Environment

- OS: Windows
- Rust: `rustc 1.93.0 (254b59607 2026-01-19)`
- Initial `where.exe clang-cl`: not found in the current shell PATH
- LLVM compiler present at: `C:\Program Files\LLVM\bin\clang-cl.exe`
- Local fix applied for this verification session:
  `$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH`

## Commands Run

- `cargo fmt --all -- --check`
- `cargo check -p agent-diva-agent --no-default-features`
- `cargo test -p agent-diva-agent test_with_toolset`
- `cargo test -p agent-diva-agent test_tool_assembly_subagent_mode_excludes_mentle_custom_tools`
- `cargo test -p agent-diva-agent subagent_does_not_receive_mentle_by_default`
- `cargo test -p agent-diva-agent test_build_agent_tools_reuses_custom_tools_with_cron`
- `cargo test -p agent-diva-agent test_register_default_tools_preserves_custom_tools_with_cron`
- `cargo test -p agent-diva-agent test_build_subagent_prompt_omits_mentle_routing`
- `cargo check -p agent-diva-agent --features mentle`
- `cargo test -p agent-diva-agent --features mentle mentle`
- `cargo test -p agent-diva-agent --features mentle test_with_tools_active_runtime_enables_registry_and_prompt`
- `cargo test -p agent-diva-agent --features mentle test_with_tools_startup_cron_preserves_mentle_custom_tools`
- `cargo test -p agent-diva-core --features mentle memory`
- repository search for registry-sourced `memtle 0.1.2`
- repository search for forbidden `memtle` `path`, `git`, or
  `[patch.crates-io]` overrides

## Result

- Formatting passed.
- Default agent check passed.
- `with_toolset()` regression tests passed, including external-registry
  isolation.
- Subagent config, registry, and prompt isolation tests passed.
- Cron/default rebuild custom-tool preservation tests passed.
- Mentle feature agent check passed after the LLVM PATH prefix was added.
- Mentle feature agent and core memory tests passed.
- Static policy checks confirmed `memtle 0.1.2` resolves from the workspace
  registry dependency and no workspace manifest overrides it through `path`,
  `git`, or `[patch.crates-io]`.

All S4-A9 and S4-A10 checks passed for this host after the current shell PATH
was updated to include LLVM.

## Tooling Note

The PowerShell session did not expose an `rg` executable. Static policy checks
were performed through the repository search tool instead. If a shell-only
reproduction is required on a host without `rg`, use PowerShell `Select-String`
against workspace `Cargo.toml` files for the same patterns.
