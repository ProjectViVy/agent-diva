# Acceptance

## Steps

1. Configure Windows sandbox level to `restricted-token`.
2. Run a simple sandboxed command such as `echo hello`.
3. Confirm the command returns output instead of reporting that Windows sandbox is unavailable or unimplemented.
4. Run `cargo check -p agent-diva-sandbox` and confirm compilation succeeds.
