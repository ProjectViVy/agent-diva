# Verification

- `cargo fmt --all`
- `cargo check -p agent-diva-cli`
- `cargo test -p agent-diva-cli tui_disables_terminal_logging`
- `cargo test -p agent-diva-cli agent_disables_terminal_logging_and_startup_branding`
- Manual smoke path attempted with `"/quit" | cargo run -p agent-diva-cli --bin agent-diva -- tui`
- Result: failed under command-captured execution with `os error 232` (`The pipe is being closed`), so final interactive confirmation still requires a real terminal session rather than redirected stdin/stdout.
