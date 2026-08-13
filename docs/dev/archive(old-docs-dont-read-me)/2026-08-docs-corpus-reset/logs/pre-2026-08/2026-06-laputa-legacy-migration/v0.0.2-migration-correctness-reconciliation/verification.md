# Verification

Date: 2026-06-17

## Commands

- `cargo fmt -p agent-diva-laputa -p agent-diva-agent -- --check`
- `cargo test -p agent-diva-laputa migration -- --nocapture`
- `cargo test -p agent-diva-agent context::tests::test_build_system_prompt_excludes_legacy_authority_sections_by_default -- --nocapture`

## Result

- Formatting passed.
- Laputa migration tests passed, including state merge, root legacy discovery, and bootstrap exclusion coverage.
- Agent context test passed, confirming `BOOTSTRAP.md` is no longer injected as runtime prompt authority.
