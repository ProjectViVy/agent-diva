# Verification

- `cargo test -p agent-diva-agent mask::mask_registry --quiet`
- `cargo test -p agent-diva-agent mask::tool_policy --quiet`
- `cargo test -p agent-diva-agent test_tool_assembly_assist_mode_is_read_only --quiet`
- `pnpm exec vue-tsc --noEmit` failed on pre-existing unrelated GUI issues.
- `cargo check -p agent-diva-gui --quiet` failed on pre-existing unrelated `agent-diva-sandbox` compile errors.
