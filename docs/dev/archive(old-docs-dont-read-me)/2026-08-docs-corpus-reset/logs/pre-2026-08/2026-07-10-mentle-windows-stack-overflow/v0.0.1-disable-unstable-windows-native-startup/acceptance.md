# Acceptance

1. Run `cargo run -p agent-diva-cli -- gateway run` on Windows with Mentle enabled in configuration.
2. Confirm gateway bootstrap completes without `STATUS_STACK_OVERFLOW`.
3. Confirm the log reports Mentle was disabled for the Windows native runtime and that Laputa/Markdown memory remains available.

