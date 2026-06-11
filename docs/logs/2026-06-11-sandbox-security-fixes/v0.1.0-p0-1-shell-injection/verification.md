# Verification

## Commands

- `cargo test -p agent-diva-sandbox manager::tests::test_sandbox_command_quotes_metacharacters`
- `cargo test -p agent-diva-sandbox manager::tests::test_to_command_string_prevents_shell_injection`

## Result

- `cargo test -p agent-diva-sandbox manager::tests::test_sandbox_command_quotes_metacharacters -- --nocapture` passed.
- `cargo test -p agent-diva-sandbox manager::tests::test_to_command_string_prevents_powershell_injection -- --nocapture` passed on the current Windows environment.
