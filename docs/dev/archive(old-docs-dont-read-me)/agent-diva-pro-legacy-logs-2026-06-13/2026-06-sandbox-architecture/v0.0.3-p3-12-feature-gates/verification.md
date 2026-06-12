# Verification

- Command: `cargo build -p agent-diva-sandbox`
- Result: passed
- Command: `cargo build -p agent-diva-sandbox --no-default-features --features manager,platform`
- Result: passed
- Command: `cargo build -p agent-diva-sandbox --all-features`
- Result: passed
- Command: `cargo test -p agent-diva-sandbox`
- Result: passed
