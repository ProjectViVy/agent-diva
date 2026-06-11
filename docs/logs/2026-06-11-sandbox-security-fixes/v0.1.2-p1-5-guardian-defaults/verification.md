# Verification

## Commands

- `cargo test -p agent-diva-sandbox guardian::tests::test_guardian_config_default`
- `cargo test -p agent-diva-sandbox guardian::tests::test_guardian_default_does_not_auto_approve_known_safe_commands`

## Result

- `cargo test -p agent-diva-sandbox guardian::tests::test_guardian_config_default -- --nocapture` passed.
- `cargo test -p agent-diva-sandbox guardian::tests::test_guardian_default_does_not_auto_approve_known_safe_commands -- --nocapture` passed.
